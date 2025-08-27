#pragma once

#include <memory>
#include <string>
#include <variant>

namespace intp::types {
    struct PrimitiveType {
        enum class Type {
            Float,
            Str,
            Custom,
            Any,
        };

        Type type;
        std::string custom;

        friend std::ostream &operator<<(std::ostream &os, const PrimitiveType &typ);
    };

    struct CompoundType;

    using Type = std::variant<PrimitiveType, std::shared_ptr<CompoundType> >;

    struct CompoundType {
        PrimitiveType l_type;
        Type r_type;

        friend std::ostream &operator<<(std::ostream &os, const CompoundType &typ);
    };

    std::ostream &operator<<(std::ostream &os, const Type &typ);
}
