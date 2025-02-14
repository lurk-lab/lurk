// Loading this file should load all library functions and the appropriate macro-env. For now, just loading macros.ls
// accomplishes this, but as the library evolves, so should this easiest entrypoint.
//
// The goal is that consuming code should just !(load "lib.ls").

load("macros.ls")
