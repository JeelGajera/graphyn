#ifndef GEOMETRY_H
#define GEOMETRY_H

struct Point {
    int x;
    int y;
};

int point_distance(struct Point a, struct Point b);
int unused_helper(void);

#endif
