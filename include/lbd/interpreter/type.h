#pragma once

#include <memory>
#include <string>
#include <variant>

namespace interpreter::type {
    struct PrimitiveType {
        enum class Type {
            Float,
            String,
            Custom,
            Any,
        };

        Type type;
        std::string custom;

        friend std::ostream &operator<<(std::ostream &stream, const PrimitiveType &type);
    };

    struct CompoundType;

    using Type = std::variant<PrimitiveType, std::shared_ptr<CompoundType> >;

    struct CompoundType {
        PrimitiveType leftType;
        Type rightType;

        friend std::ostream &operator<<(std::ostream &stream, const CompoundType &type);
    };

    std::ostream &operator<<(std::ostream &stream, const Type &type);
}
