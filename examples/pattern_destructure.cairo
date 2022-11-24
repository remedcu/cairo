enum MyEnum { a: (felt, felt), b: felt }

func foo(e: MyEnum) {
    match e {
        MyEnum::a((x, y)) => {},
        MyEnum::b(z) => {},
    }
}
