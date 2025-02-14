// Lurk doesn't (yet) support macros explicitly, so let's add a poor-man's macro compiler.
load("util.ls")

def(notImmediate, function (x) {
  if (x === nil) {
    nil
  } else if (x === t) {
    nil
  } else if (typeEqq(x, x)) {
    t
  } else if (typeEqq([0], x)) {
    t
  }
})


def(immediate, x => !(notImmediate(x)))

def(bindMacro, function (name, macroEnv, macroFun) {
  cons(cons(name, macroFun), macroEnv)
})

// macro-env is an alist of (name . macro-function).
def(macroexpand1, function (macroEnv, src) {
  if (typeEqq([cons], src)) {
    const head = car(src);
    if (typeEqq(symbol, head)) { // This excludes builtins.
      const macroFun = cdr(assoc(head, macroEnv));
      macroFun ? macroFun(src) : src
    } else {
      src
    }
  } else {
    src
  }
})

defrec(macroexpand, function (macroEnv, src) {
  const expanded = macroexpand1(macroEnv, src);
  src === expanded ? src : macroexpand(macroEnv, expanded)
})

defrec(macroexpandAll, function (macroEnv, src) {
  const expanded = macroexpand(macroEnv, src);
  if (typeEqq([cons], expanded)) {
    const head = car(expanded);
    if (head === quote.q) {
      src
      } else {
        if (member(head, [let, letrec])) {
          cons(head, cons(map(binding => list(car(binding), macroexpandAll(macroEnv, cadr(binding))),
                              cadr(expanded)),
                          macroexpandAll(macroEnv, cddr(expanded))))
        } else {
          if (member(head, [eqq, typeEqq, lambda])) {
            cons(head, cons(cadr(expanded),
                            map(macroexpandAll(macroEnv), cddr(expanded))))
          } else {
            map(macroexpandAll(macroEnv), expanded)
          }
        }
      }
  } else {
    expanded
  }
})

def(evl, function (macroEnv, env, src) {
  eval(macroexpandAll(macroEnv, src), env)
})

def(evl1, function (macroEnv, src) {
  eval(macroexpandAll(macroEnv, src))
})
