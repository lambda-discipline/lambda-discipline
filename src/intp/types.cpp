#include <lbd/error.h>
#include <lbd/intp/types.h>

namespace intp::types {
    std::ostream &operator<<(std::ostream &stream, const PrimitiveType &type) {
        switch (type.type) {
            case PrimitiveType::Type::Float:
                return stream << "Float";
            case PrimitiveType::Type::String:
                // TODO: Change this to String.
                return stream << "Str";
            case PrimitiveType::Type::Custom:
                return stream << "Custom";
            case PrimitiveType::Type::Any:
                return stream << "Any";
            default:
                UNREACHABLE("unhandled type");
        }
    }

    std::ostream &operator<<(std::ostream &stream, const CompoundType &type) {
        return stream << type.leftType << " -> " << type.rightType;
    }

    std::ostream &operator<<(std::ostream &stream, const Type &type) {
        std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, PrimitiveType>) {
                stream << arg;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<CompoundType> >) {
                stream << *arg;
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled type");
            }
        }, type);
        return stream;
    }
}
