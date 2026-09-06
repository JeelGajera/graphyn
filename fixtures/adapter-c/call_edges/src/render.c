#include "geometry.h"
#include <stdio.h>

static int scale(int value) {
    return value * 2;
}

int render(struct Point origin, struct Point target) {
    /* A call to a function defined in this file. */
    int scaled = scale(origin.x);

    /* A call reaching a function through a header *prototype*. The header
       declares it but does not define it, so there is no symbol to point at
       and no edge is recorded. */
    int distance = point_distance(origin, target);

    /* A standard-library call: nothing in this graph is named by it. */
    printf("%d\n", distance);

    return distance + scaled;
}
