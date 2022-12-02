extern type Pedersen;

extern func pedersen(a: felt, b: felt) -> felt implicits(pedersen: Pedersen) nopanic;
extern func pedersen_get_gas() -> Option::<()> implicits(rc: RangeCheck, gb: GasBuiltin) nopanic;
