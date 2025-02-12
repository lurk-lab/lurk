def(not, x => x ? nil : t)

// ;; TODO: tests, but WYSIWYG.
def(caar, x => car(car(x)))
def(cadr, x => car(cdr(x)))
def(cdar, x => cdr(car(x)))
def(cddr, x => cdr(cdr(x)))
def(caaar, x => car(car(car(x))))
def(caadr, x => car(car(cdr(x))))
def(cadar, x => car(cdr(car(x))))
def(caddr, x => car(cdr(cdr(x))))
def(cdaar, x => cdr(car(car(x))))
def(cdadr, x => cdr(car(cdr(x))))
def(cddar, x => cdr(cdr(car(x))))
def(cdddr, x => cdr(cdr(cdr(x))))

// break is reserved in JS.
def(todo, x => { emit(list(todo.key, x)); 
                 // this will error
                 BREAK()
               })

def(error, data => {
  emit(list(error.key, data))
  // this will error
  BREAK()
})

// This should be a macro, so we can include the unevaluated form in the error.
def(assert_, x => x || error(assertionFailure.key, nil))

// This should be a macro, so we can include the unevaluated form in the error.
def(ensure, x => x || fail())

// TODO: better handling of ?
defrec(member, function (x, l) {
  if (l) {
    x === car(l) ? x : member(x, cdr(l))
  }})

def(position, (elt, l) => {
  function aux(l) {
    eq(car(l), elt) ? 0 : 1 + aux(cdr(l))
  }
  if (l) aux(l)
})

defrec(nth, function (n, l) {
  if(l) (n === 0 ? car(l) : nth(n-1, cdr(l)))
})

defrec(nthCdr, (n, l) => n === 0 ? l : cdr(nthCdr(n-1, l)))

def(nth, (n, l) => car(nthCdr(n, l)))

defrec(append, (x, y) => x ? cons(car(x), append(cdr(x), y)) : y)

// More efficient version of (append (reverse x) y)
defrec(revappend, (x, y) => x ? revappend(cdr(x), cons(car(x), y)) : y)

def(getf, function (plist, indicator) {
  function aux(plist) {
    if (plist) (car(plist) === indicator ? car(cdr(plist)) : aux(cdr(cdr(plist))))
  }
  aux(plist)
})

def(setPropertyAux, function (plist, indicator) {
  function aux(acc, plist) {
    if(plist) {
      (car(plist) === indicator ?
       cons(acc, cdr(plist)) :
       aux(cons(cadr(plist), cons(car(plist), acc)),
           cddr(plist)))
    }
  }
  aux(nil, plist)
})

def(setProperty, function (plist, indicator, value) {
  const found = setPropertyAux(plist, indicator)
  found ? revappend(car(found),
                    cons(indicator, cons(value, cdr(cdr(found))))) // memoized
    : cons(indicator, cons(value, plist))
})

def(updateProperty, function (plist, indicator, updateFn) {
  const found = setPropertyAux(plist, indicator);
  found ?
    revappend(car(found),
              cons(indicator, cons(updateFn(cadr(found)), cdr(cdr(found))))) // memoized
    : cons(indicator, cons(value, plist))
})

def(foldProperties, function (f, acc, plist) {
  function aux(acc, plist) {
    cdr(plist) ? aux(f(acc, cadr(plist)), cddr(plist))
      : acc
  }
  aux(acc, plist)
})

def(mapProperties, function(f, plist) {
  function aux(plist) {
    if (cdr(plist)) {
      cons(f(cadr(plist)), aux(cddr(plist)))
    }
  }
  aux(plist)
})

def(assoc, function(item, alist) {
  function aux(alist) {
    if (alist) {
      (caar(alist) === item ? car(alist) : aux(cdr(alist)))
    }
  }
  aux(alist)
})

defrec(length, l => l ? 1 + length(cdr(l)) : 0)

defrec(reverse, l => {
  function aux(acc, l) {
    l ? aux(cons(car(l), acc), cdr(l)) : acc
  }
  aux(nil, l)
})

def(zip, function(a, b) {
  function aux(a, b) {
    if (a)
      if (b)
        cons(cons(car(a), car(b)), aux(cdr(a), cdr(b)))
  }
  aux(a, b)
})

defrec(sort, function(l) {
  if (cdr(l)) {
    const sortedCdr = sort(cdr(l));
    car(l) < car(sortedCdr) ?
      cons(car(l), sortedCdr) :
      cons(car(sortedCdr),
           sort(cons(car(l), cdr(sortedCdr))))
  } else l
})


defrec(map, function (f, l) {
  if (l) { cons(f(car(l)), map(f, cdr(l))) }
})


def(permute, function (l, seed) {
  const committed = map(elt => bignum(hide(bignum(commit(seed)), elt)), l);
  const sorted = sort(committed);
  map(c => open(c), sorted)
})

// exponentiate: b^e
defrec(expt, function(b, e) {
  if (e === 0) {
    1
  } else {
    if (e % 2 === 1) { // e is odd
      b * expt(b*b, (e-1)/2)
      } else {
        expt(b*b, e/2)
      }
  }
})

// todo: make variadic when possible
def(compose, (a, b) => ((x) => a(b(x))))

// FIXME: Remove this once #358 is fixed. This is just a workaround. The renaming is because builtin apply cannot be
// shadowed.
defrec(applyx, function(f, args) {
  if (args) {
    if (cdr(args)) {
      applyx(f(car(args)), cdr(args))
    } else {
      f(car(args))
    }
  }
})

def(fold, function (f, acc, list) {
  function aux (acc, list) {
    if (list) {
      aux(f(acc, car(list)), cdr(list))
    } else {
      acc
    }
  }
  aux(acc, list)
})
