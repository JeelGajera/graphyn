#include "shapes.hpp"

int draw(int radius) {
    // C++ construction: the one shape that is an instantiation outright.
    Circle* circle = new Circle();
    circle->radius = radius;

    // A functional cast: spelled exactly like a call, and nothing is called.
    // The resolved target's kind is what keeps it out of --kind calls.
    Circle copy = Circle(*circle);

    // A call to a function defined in an included header.
    return area(circle->radius) + copy.radius;
}
