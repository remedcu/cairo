#[contract]
mod TestContract {
    struct Storage { my_storage_var: felt, }

    func internal_func(ref system: System) -> felt {
        1
    }

    #[external]
    func test(ref system: System, ref arg: felt, arg1: felt, arg2: felt) -> felt {
        let x = my_storage_var::read(system);
        // super::__generated__TestContract::my_storage_var::write(system, x + 1);
        x + internal_func(system)
    }
}
