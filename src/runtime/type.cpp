#include <lbd/runtime/type.h>
#include <lbd/error.h>

namespace runtime::type {
    inline TypeTag typeTagFromValue(const Value &value) {
        return std::visit([&]<typename T0>(T0 &&_) -> TypeTag {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, double>) {
                return TypeTag::Float;
            } else if constexpr (std::is_same_v<T, std::string>) {
                return TypeTag::String;
            } else if constexpr (std::is_same_v<T, Closure>) {
                return TypeTag::Closure;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction> >) {
                return TypeTag::NativeFunction;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<List> >) {
                return TypeTag::List;
            } else {
                return TypeTag::Any;
            }
        }, value);
    }

    inline std::string typeTagToString(const TypeTag tag) {
        switch (tag) {
            case TypeTag::Float:
                return "Float";
            case TypeTag::String:
                return "String";
            case TypeTag::Closure:
                return "Closure";
            case TypeTag::NativeFunction:
                return "NativeFunction";
            case TypeTag::List:
                return "List";
            case TypeTag::Any:
                return "Any";
            default:
                STATIC_ASSERT(true, "unhandled type");
                return "UNKNOWN_TYPE"; // Unreachable
        }
    }

    SimpleType::SimpleType(const TypeTag tag) : tag(tag) {
    }

    bool SimpleType::matches(const Value &value) const {
        return tag == TypeTag::Any || tag == typeTagFromValue(value);
    }

    std::string SimpleType::toString() const {
        return typeTagToString(tag);
    }

    bool SimpleType::isCallable() const {
        return false;
    }

    ListType::ListType(const std::shared_ptr<Type> &elementType) : elementType(elementType) {
    }

    bool ListType::matches(const Value &value) const {
        if (typeTagFromValue(value) != TypeTag::Any) {
            return false;
        }
        auto &[elements] = *std::get<std::shared_ptr<List> >(value);
        return std::all_of(elements.begin(), elements.end(), [&](const auto &element) {
            return elementType->matches(element);
        });
    }

    std::string ListType::toString() const {
        std::ostringstream outputStringStream;
        outputStringStream << "[" << elementType->toString() << "]";
        return outputStringStream.str();
    }

    bool ListType::isCallable() const {
        return false;
    }

    FunctionType::FunctionType(const std::vector<std::shared_ptr<Type> > &argumentTypes,
                               const std::shared_ptr<Type> &returnType) : argumentTypes(argumentTypes),
                                                                          returnType(returnType) {
    }

    bool FunctionType::matches(const Value &value) const {
        // TODO: Complete this.
        return false;
    }

    std::string FunctionType::toString() const {
        std::ostringstream outputStringStream;
        for (auto &argumentType: argumentTypes) {
            if (argumentType->isCallable()) {
                outputStringStream << "(" << argumentType->toString() << ") ";
            } else {
                outputStringStream << argumentType->toString() << " ";
            }
        }
        if (returnType->isCallable()) {
            outputStringStream << "(" << returnType->toString() << ") ";
        } else {
            outputStringStream << returnType->toString() << " ";
        }
        return outputStringStream.str();
    }

    bool FunctionType::isCallable() const {
        return true;
    }

    int FunctionType::arity() const {
        // TODO: For variadic-functions return '-1'.
        return argumentTypes.size();
    }
}
