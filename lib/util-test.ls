load.meta('util.lurk');

assertError(todo(restOfOwl.key));

// error
assertError(error(nasty.key, 123));

// assert
// FIXME: This requires #422 to be fixed.
// assertEq(9, '.lurk.user.assert'.sym(9));
// assertError('.lurk.user.assert'.sym(5 === 2 + 2));

// ensure
assertEq(5, ensure(5));
// current framework doesn't allow testing this.
// assertError.meta(ensure(nil));

// member
// TODO: make this better.
assert('member?'.sym(2, [1, 2, 3]));
assert(not('member?'.sym(4, [1, 2, 3])));

// position
assertEq(2, position(c.key, [a.key, b.key, c.key, d.key]));

// nth
assertEq(c.key, nth(2, [a.key, b.key, c.key, d.key]));

// append
assertEq(nil, append(nil, nil));
assertEq([1], append([1], nil));
assertEq([1], append(nil, [1]));
assertEq([1, 2, 3, 4], append([1, 2], [3, 4]));

// revappend
assertEq([3, 2, 1, 4, 5, 6], revappend([1, 2, 3], [4, 5, 6]));
assertEq([4, 5, 6], revappend([], [4, 5, 6]))

// apply
assertEq(27, apply((x, y, z) => x * (y + z), [3, 4, 5]))

// getf
assertEq(2, getf([a.key, 1, b.key, 2, c.key, 3], b.key));
assertEq(nil, getf([a.key, 1, b.key, 2, c.key, 3], d.key));

// set-property
assertEq([a.key, 1, b.key, 2], setProperty([b.key, 2], a.key, 1));
assertEq([a.key, 1, b.key, 4, c.key, 3], setProperty([a.key, 1, b.key, 2, c.key, 3], b.key, 4));

// update-property
assertEq([a.key, 1, b.key, 6, c.key, 3], updateProperty([a.key, 1, b.key, 2, c.key, 3], b.key, (x) => 3 * x))

// fold-properties
assertEq(6, foldProperties((a, b) => a + b, 0, [a.key, 1, b.key, 2, c.key, 3]));

//  map-properties
assertEq([2, 4, 6], mapProperties((x) => 2 * x, [a.key, 1, b.key, 2, c.key, 3]));

//////////////////////////////////////

// assoc
assertEq([b.key, 2].pair, assoc(b.key, [[a.key, 1].pair, [b.key, 2].pair, [c.key, 3].pair]));
emit(cons(assocB.key, assoc(b.key, [[a.key, 1].pair, [b.key, 2].pair, [c.key, 3].pair]))) // 30 iterations

// It's probably because of the outer begin wrapping the whole file -- which affects memoization.

assertEq(nil, assoc(d.key, [[a.key, 1].pair, [b.key, 2].pair, [c.key, 3].pair]))
emit(cons(assocD.key, assoc(d.key, [[a.key, 1].pair, [b.key, 2].pair, [c.key, 3].pair]))) // 45 iterations

assertEq(nil, assoc(d.key, [[a.key, 1], [b.key, 2], [c.key, 3]]));

// length
assertEq(3, length([a, b, c]));
assertEq(0, length([]));

// reverse
assertEq([c, b, a], reverse([a, b, c]));

// ;; zip
assertEq([[a, 1].pair, [b, 2].pair, [c, 3].pair], zip([a, b, c], [1, 2, 3]));

// sort
assertEq([1, 1, 2, 4, 7], sort([7, 1, 4, 1, 2]));

// map
assertEq([1, 4, 9, 16], map((x) => x * x, [1, 2, 3, 4]));

// permute
assertEq([b, d, e, c, a], permute([a, b, c, d, e], 123));
assertEq([d, a, c, e, b], permute([a, b, c, d, e], 987));

// expt
assertEq(32, expt(2, 5));

// compose
// To avoid the IIFE here would require a non-JS-compatible surface syntax.
assertEq(36, (()=>{
                    function square(x) { x * x; }
                    function double(x) { x * 2; }
                    const doubleThenSquare = compose(square, double);

                    doubleThenSquare(3)
              })());

//  not
assertEq(t, not(nil));
assertEq(true, not(nil));
assertEq(nil, not(t));
assertEq(nil, not(true));
assertEq(nil, not(123));
assertEq(false, not(123));

// fold
assertEq(15, fold((a, b) => a + b, 0, [1, 2, 3, 4, 5]));
