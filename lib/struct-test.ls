load("struct.ls")

////////////////////////////////////////////////////////////////////////////////
// struct
def(foo, struct([a.key, b.key, c.key]))
def(x, foo('new'.key)([1, 2, 3]))

assertEq(1, x(a.key))
assertEq(2, x(b.key))
assertEq(3, x(c.key))

def(y, foo('new'.key)([9, 8, 7]))
emit(cons(fooNew.key, foo('new'.key)([9, 8, 7]))) // 14 iterations

// First access of :b in x -- 54 iterations
cons(xb.key,x(b.key))
//  [54 iterations] => (:xb . 2)

// Second access of :b in x -- 3 iterations
cons(xbXb.key, (()=> { x(b.key); x(b.key) })())
// [57 iterations] => (:xb-xb . 2)

// First access of :b in y -- 29 iterations
cons(xbXy.key,(()=> { x(b.key); y(b.key) })())
// [83 iterations] => (:xb-xy . 8)

// First access of :a in x -- 25 iterations
cons(xbYbXa.key, (()=> { x(b.key); y(b.key); x(a.key)})())
// [108 iterations] => (:xb-yb-xa . 1)

// First access of :a in y -- 9 iterations
cons(xbYbXaYa.key, (()=> { x(b.key); y(b.key); x(a.key); y(a.key)})())
// (cons :xb-yb-xa-ya (begin (x :b) (y :b) (x :a) (y :a)))
// [119 iterations] => (:xb-yb-xa-ya . 9)

////////////////////////////////////////////////////////////////////////////////
// alist-struct

def(foo, alistStruct([a.key, b.key, c.key]))
def(x, foo('new'.key)([1, 2, 3]))

assertEq(1, x(a.key))
assertEq(2, x(b.key))
assertEq(3, x(c.key))

def(y, foo('new'.key)([9, 8, 7]));
emit(cons(fooNew.key, foo('new'.key)([9, 8, 7]))) // 65 iterations

// First access of :b in x -- 32 iterations
cons(xb.key, x(b.key))
// [40 iterations] => (:xb . 2)

// Second access of :b in x -- 3 iterations
cons(xbXb.key, (()=> { x(b.key); x(b.key) })())
// [43 iterations] => (:xb-xb . 2)

// First access of :b in y -- 37 iterations
cons(xbXy.key, (()=> { x(b.key); y(b.key) })())
// [80 iterations] => (:xb-xy . 8)

// First access of :a in x -- 21 iterations
cons(xbYbXa.key, (()=> { x(b.key); y(b.key); x(a.key) })())
// [101 iterations] => (:xb-yb-xa . 1)

// First access of :a in y -- 20 iterations
cons(xbYbXaYa.key, (()=> { x(b.key); y(b.key); x(a.key); y(a.key)})())
// [121 iterations] => (:xb-yb-xa-ya . 9)

