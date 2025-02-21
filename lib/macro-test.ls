load("macro.ls")

assert(immediate(123))
assert(immediate(123n))
assert(immediate(nil))
assert(immediate(t))
assert(immediate(asdf.key))
assert(immediate("frog"))
assert(immediate('q'.char))
assert(!immediate(cons(1,2)))
assert(!immediate(sym.q))
assert(immediate(()=>nil))

def(macroEnv, [])
def(macroEnv, bindMacro(and.q, macroEnv,
                        function (whole) {
                          if (cdr(whole)) {
                            if (cdr(cdr(whole))) {
                              list('if'.q, car(cdr(whole)),
                                   cons(and.q, cdr(cdr(whole))))  // this is memoized
                            } else {
                              car(cdr(whole))
                            }
                          } else {
                            t
                          }}))

def(macroEnv, bindMacro(or.q, macroEnv,
                        function (whole) {
                          if (cdr(whole)) {
                            if (cdr(cdr(whole))) {
                              list('if'.q, car(cdr(whole)),
                                   car(cdr(whole)), // this is memoized
                                   cons(or.q, cdr(cdr(whole))))
                              } else {
                                car(cdr(whole))
                              }
                          } else {
                            nil
                          }}))

def(macroEnv, bindMacro(foo.q, macroEnv,
                        function (whole)
                        { cons(bar.q, cdr(whole)) }))

def(macroEnv, bindMacro(bar.q, macroEnv,
                        function (whole) {
                          cons(baz.q, cdr(whole))
                          }))

assertEq(123, macroexpand1([], 123))

assertEq(['if'.sym, a, and(b, c)], macroexpand1(macroEnv, quote(and(a, b, c))))
assertEq(t, macroexpand1(macroEnv, [and]))

assertEq(['if'.sym, a, a, or(b, c)], macroexpand1(macroEnv, quote(or(a, b, c))))
assertEq(nil, macroexpand1(macroEnv, [or]))

assertEq(quote(bar(1, 2, 3)), macroexpand1(macroEnv, quote(foo(1, 2, 3))))
assertEq(quote(baz(1, 2, 3)), macroexpand(macroEnv, quote(foo(1, 2, 3))))

assertEq(['if'.sym, a, ['if'.sym, b, c]], macroexpandAll(macroEnv, quote(and(a, b, c))))
assertEq(['if'.sym, a, a, ['if'.sym, b, b, c]], macroexpandAll(macroEnv, quote(or(a, b, c))))
assertEq([list, baz(1)], macroexpandAll(macroEnv, [list, bar(1)]))
// Don't macroexpand quoted.
assertEq([quote, bar(1)], macroexpandAll(macroEnv, [quote, bar(1)]))

assertEq(
  ['let'.sym, [[a, baz(1, 2, 3)],
               [b, [letrec, [[c, baz(9, 8, 7)]], c]]],
   [begin,
    [lambda, [x, y, z], cons(9, baz(11))],
    'if'.sym('if'.sym(a, 'if'.sym(b, c)),
             'if'.sym(a, 'if'.sym(b, c)),
             'if'.sym(baz(x),
                      baz(y),
                      'if'.sym(baz(x), baz(y))))]],
  macroexpandAll(macroEnv, ['let'.sym, [[a, foo(1, 2, 3)],
                                        [b, [letrec, [[c, bar(9, 8, 7)]],
                                             c]]],
                            [begin,
                             [lambda, [x, y, z], cons(9, foo(11))],
                             or(and(a, b, c),
                                'if'.sym(foo(x), bar(y), and(bar(x), foo(y))))]]))

def(qux, evl1(macroEnv, [lambda, [a, b, c],
                         or(and(a, b),
                            and(a, c),
                            123)]))

assertEq(123, qux(nil, b.key, c.key))
assertEq(b.key, qux(a.key, b.key, c.key))
assertEq(c.key, qux(a.key, nil, c.key))
assertEq(123, qux(nil, nil, nil))

assertEq([typeEqq, foo(x.key), baz(123)], macroexpandAll(macroEnv, [typeEqq, foo(x.key), foo(123)]))
assertEq([eqq, foo(x.key), baz(123)], macroexpandAll(macroEnv, [eqq, foo(x.key), foo(123)]))
