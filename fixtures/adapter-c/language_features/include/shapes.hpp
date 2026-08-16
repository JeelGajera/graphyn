#pragma once

namespace geometry {

class Shape {
public:
    virtual double area() const = 0;
    const char *label;
};

class Circle : public Shape {
public:
    double radius;
    double area() const override;
};

}
