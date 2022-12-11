use array::array_at;

 func test(ref syscall_ptr: SyscallPtr, mut input: Array::<felt>) -> Array::<felt> {
     match array_at::<felt>(input, integer::uint128_from_felt(0)) {
         Option::Some(x) => {
             array_new::<felt>()
         },
         Option::None(x) => {
             input
         },
     }
 }
