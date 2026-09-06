#ifndef SHAPES_HPP
#define SHAPES_HPP

class Circle {
public:
    int radius;
};

inline int area(int radius) {
    return radius * radius * 3;
}

#endif
