#pragma once

#include <string>
#include <memory>
#include <vector>

/// These are runtime-types different from frontend::type.

/// Forward declaration of value-structure, cannot import directly due to circular-dependency.
namespace runtime {
    struct Value;
}

namespace runtime::type {
    enum class TypeTag {
        Float,
        String,
        Closure,
        NativeFunction,
        List,
        Any
    };

    TypeTag typeTagFromValue(const Value &value);

    std::string typeTagToString(TypeTag tag);

    struct Type {
        virtual ~Type() = default;

        [[nodiscard]] virtual bool matches(const Value &value) const;

        [[nodiscard]] virtual bool equals(const Type &otherType) const;

        [[nodiscard]] virtual std::string toString() const = 0;

        [[nodiscard]] virtual bool isCallable() const;
    };

    struct SimpleType final : Type {
        explicit SimpleType(TypeTag tag);

        [[nodiscard]] bool matches(const Value &value) const override;

        [[nodiscard]] bool equals(const Type &otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;

    private:
        TypeTag tag;
    };

    struct ListType final : Type {
        explicit ListType(const std::shared_ptr<Type> &elementType);

        [[nodiscard]] bool matches(const Value &value) const override;

        [[nodiscard]] bool equals(const Type &otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;

    private:
        std::shared_ptr<Type> elementType;
    };

    struct FunctionType final : Type {
        // TODO: Add support for void return-type.
        explicit FunctionType(const std::vector<std::shared_ptr<Type> > &argumentTypes,
                              const std::shared_ptr<Type> &returnType, bool isVariadic = false);

        [[nodiscard]] bool matches(const Value &value) const override;

        [[nodiscard]] bool equals(const Type &otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;

        [[nodiscard]] int arity() const;

    private:
        std::vector<std::shared_ptr<Type> > argumentTypes;
        std::shared_ptr<Type> returnType;
        bool isVariadic;
        std::shared_ptr<Type> variadicType;
    };
}
