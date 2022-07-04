use crate::error::Error;
use crate::graph::*;
use crate::scope_state::*;
use std::collections::HashMap;
use Result::*;

mod arithmetic;
mod gas_station;
mod jump_nz;
mod match_nullable;
mod unconditional_jump;

trait InvokeExtension {
    fn get_effects(self: &Self, invc: &Invocation) -> Result<ScopeChange, Error>;
}

trait JumpExtension {
    fn get_effects(self: &Self, jump: &JumpInfo) -> Result<HashMap<usize, ScopeChange>, Error>;
}

pub(self) struct ExtensionRegistry {
    invoke_libcalls: HashMap<String, Box<dyn InvokeExtension + Sync + Send>>,
    jump_libcalls: HashMap<String, Box<dyn JumpExtension + Sync + Send>>,
}

lazy_static! {
    static ref REGISTRY: ExtensionRegistry = {
        let mut registry = ExtensionRegistry {
            invoke_libcalls: HashMap::<String, Box<dyn InvokeExtension + Sync + Send>>::new(),
            jump_libcalls: HashMap::<String, Box<dyn JumpExtension + Sync + Send>>::new(),
        };
        arithmetic::register(&mut registry);
        unconditional_jump::register(&mut registry);
        jump_nz::register(&mut registry);
        match_nullable::register(&mut registry);
        gas_station::register(&mut registry);
        registry
    };
}

pub fn get_invoke_effects(invc: &Invocation) -> Result<ScopeChange, Error> {
    match REGISTRY.invoke_libcalls.get(&invc.libcall.name) {
        Some(ext) => ext.get_effects(invc),
        _ => Err(Error::UnsupportedLibCallName),
    }
}

pub fn get_jump_effects(jump: &JumpInfo) -> Result<HashMap<usize, ScopeChange>, Error> {
    match REGISTRY.jump_libcalls.get(&jump.libcall.name) {
        Some(ext) => ext.get_effects(jump),
        _ => Err(Error::UnsupportedLibCallName),
    }
}
