#pragma once
#include <exception>

struct ControlledExit final : std::exception {
    const char *what() const noexcept override {
        return "Controlled non-fatal exit";
    }
};
