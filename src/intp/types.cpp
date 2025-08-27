#include <lbd/error.h>
#include <lbd/intp/types.h>

namespace intp::types {
    std::ostream &operator<<(std::ostream &os, const PrimitiveType &typ) {
        switch (typ.type) {
            case PrimitiveType::Type::Float:
                return os << "Float";
            case PrimitiveType::Type::Str:
                return os << "Str";
            case PrimitiveType::Type::Custom:
                return os << "Custom";
            case PrimitiveType::Type::Any:
                return os << "Any";
            default:
                UNREACHABLE("unhandled type");
        }
    }

    std::ostream &operator<<(std::ostream &os, const CompoundType &typ) {
        return os << typ.l_type << " -> " << typ.r_type;
    }

    std::ostream &operator<<(std::ostream &os, const Type &typ) {
        std::visit([&]<typename T0>(T0 &&arg) {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, PrimitiveType>) {
                os << arg;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<CompoundType> >) {
                os << *arg;
            } else {
                STATIC_ASSERT_UNREACHABLE_T(T, "unhandled type");
            }
        }, typ);
        return os;
    }
}
