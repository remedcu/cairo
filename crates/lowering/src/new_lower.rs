use std::collections::{HashMap, HashSet};

use defs::ids::{FreeFunctionId, LanguageElementId};
use diagnostics::Diagnostics;
use id_arena::Arena;
use itertools::{zip_eq, Itertools};
use semantic::items::enm::SemanticEnumEx;
use semantic::items::imp::ImplLookupContext;
use semantic::{ConcreteTypeId, Mutability, TypeLongId, VarId};
use utils::{extract_matches, try_extract_matches};

use crate::db::LoweringGroup;
use crate::diagnostic::{LoweringDiagnostic, LoweringDiagnostics};
use crate::lower::new_context::{LoweringContext, LoweringFlowError};
use crate::new_objects::{
    Block, BlockId, LoweredStatement, MatchArm, StatementLiteral, StatementMatchEnum, Variable,
    VariableId,
};

/// A lowered function code.
#[derive(Debug, PartialEq, Eq)]
pub struct LoweredFreeFunction {
    /// Diagnostics produced while lowering.
    pub diagnostics: Diagnostics<LoweringDiagnostic>,
    /// Block id for the start of the lowered function.
    pub root: BlockId,
    /// Arena of allocated lowered variables.
    pub variables: Arena<Variable>,
    /// Arena of allocated lowered blocks.
    pub blocks: Arena<Block>,
}

/// Lowers a semantic free function.
pub fn lower_free_function(
    db: &dyn LoweringGroup,
    free_function_id: FreeFunctionId,
) -> Option<LoweredFreeFunction> {
    log::trace!("Started new lowering of a free function.");
    let function_def = db.free_function_definition(free_function_id)?;
    let generic_params = db.free_function_declaration_generic_params(free_function_id)?;
    let signature = db.free_function_declaration_signature(free_function_id)?;

    let implicits = db.free_function_all_implicits_vec(free_function_id)?;
    // Params.
    let ref_params = signature
        .params
        .iter()
        .filter(|param| param.mutability == Mutability::Reference)
        .map(|param| VarId::Param(param.id))
        .collect_vec();
    let input_semantic_vars: Vec<semantic::Variable> =
        signature.params.into_iter().map(semantic::Variable::Param).collect();
    let (input_semantic_var_ids, input_var_tys): (Vec<_>, Vec<_>) = input_semantic_vars
        .iter()
        .map(|semantic_var| (semantic_var.id(), semantic_var.ty()))
        .unzip();
    // let input_var_tys = chain!(implicits.clone(), input_var_tys).collect();

    let implicits_ref = &implicits;
    let mut ctx = LoweringContext {
        db,
        function_def: &function_def,
        diagnostics: LoweringDiagnostics::new(free_function_id.module(db.upcast())),
        variables: Arena::default(),
        blocks: Arena::default(),
        lookup_context: ImplLookupContext {
            module_id: free_function_id.module(db.upcast()),
            extra_modules: vec![],
            generic_params,
        },
    };

    // Fetch body block expr.
    // TODO(yg): try_extract_matches.
    let semantic_block =
        extract_matches!(&function_def.exprs[function_def.body], semantic::Expr::Block);

    let mut scope = &mut LoweringBlockScope::default();
    lower_block(&mut ctx, &mut scope, semantic_block);
    let root_block = ctx.blocks.alloc(Block { statements: scope.statements });

    Some(LoweredFreeFunction {
        diagnostics: ctx.diagnostics.build(),
        root: root_block,
        variables: ctx.variables,
        blocks: ctx.blocks,
    })
}

// TODO(yg): move to new_scope.rs.
/// The scope of a lowered block while it's being lowered.
#[derive(Default)]
pub struct LoweringBlockScope {
    /// Mapping from semantic vars to their lowered var ID.
    vars: HashMap<semantic::VarId, VariableId>,
    /// Mapping of variables that were required but not present to their initial allocated variable
    /// ID.
    required_vars: HashMap<semantic::VarId, VariableId>,
    statements: Vec<LoweredStatement>,
}

/// Lowers a match-arm (or if) block or a function's block. Only these blocks are represented in the
/// lowering output, as simple blocks are trivial (todo(yg): rephrase the end?).
pub fn lower_block(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr_block: &semantic::ExprBlock,
) {
    for stmt_id in expr_block.statements.iter() {
        let stmt = &ctx.function_def.statements[*stmt_id];
        match stmt {
            semantic::Statement::Expr(stmt_expr) => {
                lower_expr(ctx, scope, &stmt_expr.expr);
            }
            semantic::Statement::Let(stmt_let) => {
                lower_expr(ctx, scope, &stmt_let.expr);
            }
            // TODO(yg):
            semantic::Statement::Return(_) => {}
        }
    }
}

// TODO(yg): doc all

fn get_pattern_vars(_pattern: &semantic::Pattern) -> Vec<semantic::VarId> {
    // TODO(yg)
    vec![]
}

fn lower_expr_match(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr_match: &semantic::ExprMatch,
) {
    let mut match_arms = Vec::new();
    // TODO(yg): change unwrap to ? and result.
    let (concrete_enum_id, concrete_variants) = extract_concrete_enum(ctx, expr_match).unwrap();

    // TODO(yg): make sure the order is consistent between different runs, and between inputs and
    // outputs.
    let inputs: HashSet<VariableId> = HashSet::new();
    let outputs: HashSet<VariableId> = HashSet::new();
    for (variant, arm) in zip_eq(concrete_variants, &expr_match.arms) {
        let mut arm_scope = LoweringBlockScope::default();
        lower_expr(ctx, &mut arm_scope, &arm.expression);

        let mut arm_mapping = HashMap::new();
        for (required_var, initial_lowered_id) in arm_scope.required_vars {
            match scope.vars.entry(required_var) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    let scope_lowered_var_id = entry.get();
                    arm_mapping.insert(*scope_lowered_var_id, initial_lowered_id);
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    // TODO(yg): diagnostic, missing var...
                }
            }
        }

        // TODO(yg): 1. Is it better like this or adding one by one when inserting to arm_mapping.
        // TODO(yg): 2. Do we even need inputs+outputs in StatementMatchEnum if they can be
        // concluded from arms?
        inputs.extend(arm_mapping.keys());
        outputs.extend(arm_mapping.values());

        let block_id = ctx.blocks.alloc(Block { statements: arm_scope.statements });
        match_arms.push(MatchArm { variant, block_id, var_mapping: arm_mapping });
    }

    scope.statements.push(LoweredStatement::MatchEnum(StatementMatchEnum {
        concrete_enum: concrete_enum_id,
        inputs: inputs.into_iter().collect(),
        arms: match_arms,
        outputs: outputs.into_iter().collect(),
    }));
}

fn lower_expr(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr_id: &semantic::ExprId,
) {
    let expr = &ctx.function_def.exprs[*expr_id];
    // TODO(yg): complete all arms.
    match expr {
        semantic::Expr::Tuple(expr) => lower_expr_tuple(ctx, scope, expr),
        semantic::Expr::Assignment(expr) => lower_expr_assignment(ctx, scope, expr),
        semantic::Expr::Block(expr_block) => lower_block(ctx, scope, &expr_block),
        semantic::Expr::FunctionCall(expr) => lower_expr_function_call(ctx, scope, expr),
        semantic::Expr::Match(expr_match) => lower_expr_match(ctx, scope, expr_match),
        semantic::Expr::If(expr) => lower_expr_if(ctx, scope, expr),
        semantic::Expr::Var(v) => {
            if !scope.vars.contains_key(&v.var) {
                let lowered_id = introduce_new_var(ctx, v.ty);
                scope.required_vars.insert(v.var, lowered_id);
                scope.vars.insert(v.var, lowered_id);
            }
        }
        semantic::Expr::Literal(expr) => lower_expr_literal(ctx, scope, expr),
        semantic::Expr::MemberAccess(expr) => lower_expr_member_access(ctx, scope, expr),
        semantic::Expr::StructCtor(expr) => lower_expr_struct_ctor(ctx, scope, expr),
        semantic::Expr::EnumVariantCtor(expr) => lower_expr_enum_ctor(ctx, scope, expr),
        semantic::Expr::PropagateError(expr) => lower_expr_error_propagate(ctx, scope, expr),
        semantic::Expr::Missing(_) => {
            // TODO(yg): error? Need to return result.
        }
    }
}

fn lower_expr_tuple(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprTuple,
) {
    // TODO(yg)
}

fn lower_expr_assignment(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprAssignment,
) {
    // TODO(yg)
}

fn lower_expr_function_call(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprFunctionCall,
) {
    // TODO(yg)
}

fn lower_expr_if(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprIf,
) {
    // TODO(yg)
}

fn lower_expr_member_access(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprMemberAccess,
) {
    // TODO(yg)
}

fn lower_expr_struct_ctor(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprStructCtor,
) {
    // TODO(yg)
}

fn lower_expr_enum_ctor(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprEnumVariantCtor,
) {
    // TODO(yg)
}

fn lower_expr_error_propagate(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprPropagateError,
) {
    // TODO(yg)
}

fn lower_expr_literal(
    ctx: &mut LoweringContext<'_>,
    scope: &mut LoweringBlockScope,
    expr: &semantic::ExprLiteral,
) {
    let lowered_id = introduce_new_var(ctx, expr.ty);
    scope.statements.push(LoweredStatement::Literal(StatementLiteral {
        value: expr.value.clone(),
        output: lowered_id,
    }));
}

// TODO(yg): copied from lower.rs as is...
// TODO(yg): move?
/// Extracts concrete enum and variants from a match expression. Assumes it is indeed a concrete
/// enum.
fn extract_concrete_enum(
    ctx: &mut LoweringContext<'_>,
    expr: &semantic::ExprMatch,
) -> Result<(semantic::ConcreteEnumId, Vec<semantic::ConcreteVariant>), LoweringFlowError> {
    let concrete_ty = try_extract_matches!(
        ctx.db.lookup_intern_type(ctx.function_def.exprs[expr.matched_expr].ty()),
        TypeLongId::Concrete
    )
    .ok_or(LoweringFlowError::Failed)?;
    let concrete_enum_id =
        try_extract_matches!(concrete_ty, ConcreteTypeId::Enum).ok_or(LoweringFlowError::Failed)?;
    let enum_id = concrete_enum_id.enum_id(ctx.db.upcast());
    let variants = ctx.db.enum_variants(enum_id).ok_or(LoweringFlowError::Failed)?;
    let concrete_variants = variants
        .values()
        .map(|variant_id| {
            let variant =
                ctx.db.variant_semantic(enum_id, *variant_id).ok_or(LoweringFlowError::Failed)?;

            ctx.db
                .concrete_enum_variant(concrete_enum_id, &variant)
                .ok_or(LoweringFlowError::Failed)
        })
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(expr.arms.len(), concrete_variants.len(), "Wrong number of arms.");
    Ok((concrete_enum_id, concrete_variants))
}

/// Introduces a new variable.
pub fn introduce_new_var(ctx: &mut LoweringContext<'_>, ty: semantic::TypeId) -> VariableId {
    let ty_info = ctx.db.type_info(ctx.lookup_context.clone(), ty).unwrap_or_default();
    ctx.variables.alloc(Variable {
        duplicatable: ty_info.duplicatable,
        droppable: ty_info.droppable,
        ty,
    })
}
