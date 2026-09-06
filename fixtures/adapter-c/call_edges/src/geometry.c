#include "geometry.h"

int point_distance(struct Point a, struct Point b) {
    int dx = a.x - b.x;
    int dy = a.y - b.y;
    return dx * dx + dy * dy;
}

int unused_helper(void) {
    return 0;
}
