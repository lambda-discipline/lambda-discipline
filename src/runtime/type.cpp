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

    bool ListType::matches(const Value &value) const {
        return typeTagFromValue(value) == TypeTag::List;
    }

    bool ListType::equals(const Type &otherType) const {
        return dynamic_cast<const ListType *>(&otherType) != nullptr;
    }

    std::string ListType::toString() const {
        return "[]";
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

    const Type &FunctionType::getReturnType() const {
        return *returnType;
    }


    bool FunctionType::matches(const Value &value) const {
        // Only type-check function-applications, cannot type-check
        // closures/lambda-expressions due to lazy-evaluation.
        if (!std::holds_alternative<std::shared_ptr<NativeFunction> >(value)) {
            return false;
        }
        return equals(*std::get<std::shared_ptr<NativeFunction> >(value)->getSignature());
    }

    bool FunctionType::matchesArgumentTypes(const std::vector<Value> &values) const {
        // Perform size-check on non-variadic-functions.
        if (!isVariadic && argumentTypes.size() != values.size()) {
            return false;
        }
        for (size_t index = 0; index < argumentTypes.size(); index++) {
            if (!argumentTypes[index] || !argumentTypes[index]->matches(values[index])) {
                return false;
            }
        }
        return true;
    }

    bool FunctionType::matchesReturnType(const Value &value) const {
        // If the return-type is nullptr, i.e., return-type is void.
        // Do not perform any check in such scenario.
        return returnType ? returnType->matches(value) : true;
    }

    bool FunctionType::equals(const Type &otherType) const {
        const auto *otherFunctionType = dynamic_cast<const FunctionType *>(&otherType);
        if (!otherFunctionType) {
            return false;
        }
        // Perform size-check on non-variadic-functions.
        if (!isVariadic && argumentTypes.size() != otherFunctionType->argumentTypes.size()) {
            return false;
        }
        for (size_t index = 0; index < argumentTypes.size(); index++) {
            const auto &leftType = isVariadic ? variadicType : argumentTypes[index];
            // ReSharper disable once CppTooWideScopeInitStatement
            const auto &rightType = otherFunctionType->argumentTypes[index];
            if (!leftType || !rightType || !leftType->equals(*rightType)) {
                return false;
            }
        }
        if (returnType) {
            return otherFunctionType->returnType && returnType->equals(*otherFunctionType->returnType);
        }
        // Return-Type is nullptr i.e., void.
        return true;
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

    std::shared_ptr<Type> typeFromValue(const Value &value) {
        return std::visit([&]<typename T0>(T0 &&) -> std::shared_ptr<Type> {
            using T = std::decay_t<T0>;
            if constexpr (std::is_same_v<T, double>) {
                return std::make_shared<SimpleType>(TypeTag::Float);
            } else if constexpr (std::is_same_v<T, std::string>) {
                return std::make_shared<SimpleType>(TypeTag::String);
            } else if constexpr (std::is_same_v<T, Closure>) {
                return std::make_shared<SimpleType>(TypeTag::Closure);
            } else if constexpr (std::is_same_v<T, std::shared_ptr<NativeFunction> >) {
                const auto &nativeFunction = *std::get<std::shared_ptr<NativeFunction> >(value);
                return std::make_shared<FunctionType>(*nativeFunction.getSignature());
            } else if constexpr (std::is_same_v<T, std::shared_ptr<List> >) {
                return std::make_shared<ListType>();
            } else {
                return std::make_shared<SimpleType>(TypeTag::Any);
            }
        }, value);
    }
}
