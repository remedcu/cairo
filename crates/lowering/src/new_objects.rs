use std::collections::HashMap;

use id_arena::Id;
use num_bigint::BigInt;
use semantic::{ConcreteEnumId, ConcreteVariant};

/// A block of statements. Each block is composed of a linear sequence of statements.
/// A block may end with a `return`, which exits the current function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Statements sequence running one after the other in the block, in a linear flow.
    /// Note: Inner blocks might end with a `return`, which will exit the function in the middle.
    /// Note: Match is a possible statement, which means it has control flow logic inside, but
    /// after its execution is completed, the flow returns to the following statement of the block.
    pub statements: Vec<LoweredStatement>,
    // /// Describes how this block ends: returns to the caller or exits the function.
    // pub end: BlockEnd,
}
pub type BlockId = Id<Block>;

/// Lowered variable representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    /// Can the type be (trivially) dropped.
    pub droppable: bool,
    /// Can the type be (trivially) duplicated.
    pub duplicatable: bool,
    /// Semantic type of the variable.
    pub ty: semantic::TypeId,
}
pub type VariableId = Id<Variable>;

/// Describes what happens to the program flow at the end of a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockEnd {
    /// This block returns to the call-site, outputting variables to the call-site.
    Callsite(Vec<VariableId>),
    /// This block ends with a `return` statement, exiting the function.
    Return(Vec<VariableId>),
    /// The last statement ended the flow (e.g., match will all arms ending in return),
    /// and the end of this block is unreachable.
    Unreachable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoweredStatement {
    // Values.
    // TODO(spapini): Consts.
    Literal(StatementLiteral),

    // Flow control.
    Call(StatementCall),
    CallBlock(StatementCallBlock),
    MatchExtern(StatementMatchExtern),

    // Structs (including tuples).
    StructConstruct(StatementStructConstruct),
    StructDestructure(StatementStructDestructure),

    // Enums.
    EnumConstruct(StatementEnumConstruct),
    MatchEnum(StatementMatchEnum),
}

/// A statement that binds a literal value to a variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementLiteral {
    /// The value of the literal.
    pub value: BigInt,
    /// The variable to bind the value to.
    pub output: VariableId,
}

/// A statement that calls a user function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementCall {
    /// A function to "call".
    pub function: semantic::FunctionId,
    /// Living variables in current scope to move to the function, as arguments.
    pub inputs: Vec<VariableId>,
    /// New variables to be introduced into the current scope from the function outputs.
    pub outputs: Vec<VariableId>,
}

/// A statement that jumps to another block. If that block ends with a BlockEnd::CallSite, the flow
/// returns to the statement following this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementCallBlock {
    /// A block to "call".
    pub block: BlockId,
    /// New variables to be introduced into the current scope, moved from the callee block outputs.
    pub outputs: Vec<VariableId>,
}

/// A statement that calls an extern function with branches, and "calls" a possibly different block
/// for each branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementMatchExtern {
    // TODO(spapini): ConcreteExternFunctionId once it exists.
    /// A concrete external function to call.
    pub function: semantic::FunctionId,
    /// Living variables in current scope to move to the function, as arguments.
    pub inputs: Vec<VariableId>,
    /// Match arms. All blocks should have the same rets.
    pub arms: Vec<BlockId>,
    /// New variables to be introduced into the current scope from the arm outputs.
    pub outputs: Vec<VariableId>,
}

/// A statement that construct a variant of an enum with a single argument, and binds it to a
/// variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementEnumConstruct {
    pub variant: ConcreteVariant,
    /// A living variable in current scope to wrap with the variant.
    pub input: VariableId,
    /// The variable to bind the value to.
    pub output: VariableId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    variant: ConcreteVariant,
    block_id: BlockId,
    pub var_mapping: HashMap<VariableId, VariableId>,
}

/// A statement that matches an enum, and "calls" a possibly different block for each branch.
// TODO(yg): all the info in inputs and outputs is in arms. Consider removing and adding inputs()
// and outputs() methods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementMatchEnum {
    pub concrete_enum: ConcreteEnumId,
    /// A living variable in current scope to match on.
    pub inputs: Vec<VariableId>,
    /// Match arms. All blocks should have the same rets.
    pub arms: Vec<MatchArm>,
    /// New variables to be introduced into the current scope from the arm outputs.
    pub outputs: Vec<VariableId>,
}

/// A statement that constructs a struct (tuple included) into a new variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementStructConstruct {
    pub inputs: Vec<VariableId>,
    /// The variable to bind the value to.
    pub output: VariableId,
}

/// A statement that destructures a struct (tuple included), introducing its elements as new
/// variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementStructDestructure {
    /// A living variable in current scope to destructure.
    pub input: VariableId,
    /// The variables to bind values to.
    pub outputs: Vec<VariableId>,
}
