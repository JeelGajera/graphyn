#include "../include/shapes.hpp"

using Figure = geometry::Circle;

double render(Figure *figure) {
    return figure->radius + figure->area();
}
