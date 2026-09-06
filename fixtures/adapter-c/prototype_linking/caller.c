#include "api.h"

static int shadowed(void) {
    return 5;
}

int run(void) {
    /* Links to the definition in handler.c, across translation units. */
    int a = handle();

    /* Ambiguous and unanchored: neither records an edge. */
    int b = dispatch();
    int c = orphan();

    return a + b + c + shadowed();
}
