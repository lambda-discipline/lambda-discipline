#include <variant>
#include <lbd/error.h>
#include <lbd/runtime/interpreter.h>
#include <lbd/runtime/type.h>

namespace runtime::type {
    inline TypeTag typeTagFromValue(const Value &value) {
        return std::visit([&]<typename T0>(T0 &&) -> TypeTag {
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

    bool SimpleType::equals(const Type &otherType) const {
        if (const auto *otherSimpleType = dynamic_cast<const SimpleType *>(&otherType)) {
            return tag == otherSimpleType->tag;
        }
        return false;
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

    bool ListType::equals(const Type &otherType) const {
        if (const auto *otherListType = dynamic_cast<const ListType *>(&otherType)) {
            return elementType && otherListType->elementType && elementType->equals(*otherListType->elementType);
        }
        return false;
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
                               const std::shared_ptr<Type> &returnType,
                               const bool isVariadic) : argumentTypes(argumentTypes),
                                                        returnType(returnType), isVariadic(isVariadic) {
        if (isVariadic) {
            if (argumentTypes.size() != 1) {
                // TODO: Throw error
            }
            variadicType = argumentTypes[0];
        }
    }

    bool FunctionType::matches(const Value &value) const {
        // Only type-check function-applications, cannot type-check
        // closures/lambda-expressions due to lazy-evaluation.
        // TODO: Complete
        return false;
    }

    bool FunctionType::equals(const Type &otherType) const {
        if (const auto *otherFunctionType = dynamic_cast<const FunctionType *>(&otherType)) {
            // TODO: Check for variadic.
            if (argumentTypes.size() != otherFunctionType->argumentTypes.size()) {
                return false;
            }
            for (size_t i = 0; i < argumentTypes.size(); i++) {
                if (!(argumentTypes[i] && otherFunctionType->argumentTypes[i])) {
                    return false;
                }
                if (!argumentTypes[i]->equals(*otherFunctionType->argumentTypes[i])) {
                    return false;
                }
            }
            if (!(returnType && otherFunctionType->returnType)) {
                return false;
            }
            if (!returnType->equals(*otherFunctionType->returnType)) {
                return false;
            }
            return true;
        }
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
        if (isVariadic) {
            return -1;
        }
        return static_cast<int>(argumentTypes.size());
    }
}
