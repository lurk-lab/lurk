load("macros.ls")

// and
assertEq(['if'.sym, a, and(b, c)], macroexpand1(macroEnv, [and, a, b, c]))
assertEq(t, macroexpand1(macroEnv, [and]))

// or
assertEq(['if'.sym, a, a, or(b, c)], macroexpand1(macroEnv, [or, a, b, c]))
assertEq(nil, macroexpand1(macroEnv, [or]))

assertEq(['if'.sym, a, 'if'.sym(b, c)], macroexpandAll(macroEnv, [and, a, b, c]))
assertEq(['if'.sym, a, a, 'if'.sym(b, b, c)], macroexpandAll(macroEnv, [or, a, b, c]))

// cond
assertEq(['if'.sym, a, 1, 'if'.sym(b, 2, 'if'.sym(t, 3, nil))], macroexpandAll(macroEnv, [cond, [a, 1], [b, 2], [t, 3]]))

// typecase
assertEq([cond, [typeEqq(0, x), 123],
                [typeEqq('x'.char, x), 321]],
         macroexpand1(macroEnv, [typecase, x, [0, 123], ['x'.char, 321]]))

// case
assertEq([cond,
          [eqq(0, x), 123],
          [eqq('x'.char, x), 321]],
         macroexpand1(macroEnv, ['case'.sym, x,
                                 [0, 123],
                                 ['x'.char, 321]]))
