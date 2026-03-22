#pragma once

#include <memory>
#include <string>
#include <vector>

// These are runtime-types different from frontend::type.

// Forward declaration of value-structure, cannot import directly due to circular-dependency.
namespace lbd::runtime
{
    struct Value;
}

// TODO: Maybe move instead of copying the reference.

namespace lbd::runtime::type
{
    enum class TypeTag
    {
        Float,
        String,
        Closure,
        NativeFunction,
        List,
        Any
    };

    [[nodiscard]] TypeTag typeTagFromValue(const Value& value);

    [[nodiscard]] std::string typeTagToString(TypeTag tag);

    // TODO: Implement move-constructor and possibly move-assignment-operator.
    // TODO: Overload operator<< for printing.
    struct Type
    {
        virtual ~Type() = default;

        [[nodiscard]] virtual bool matches(const Value& value) const = 0;

        [[nodiscard]] virtual bool equals(const Type& otherType) const = 0;

        [[nodiscard]] virtual std::string toString() const = 0;

        [[nodiscard]] virtual bool isCallable() const = 0;

        [[nodiscard]] bool allowsHardCheck() const;

    protected:
        /// Do not force the thunk for type-checking if disabled, check using the thunk's unevaluated-expression.
        bool hardCheck = true;
    };

    struct SimpleType final : Type
    {
        explicit SimpleType(TypeTag tag, bool hardCheck = true);

        ~SimpleType() override = default;

        [[nodiscard]] bool matches(const Value& value) const override;

        [[nodiscard]] bool equals(const Type& otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;

    private:
        TypeTag tag;
    };

    // TODO: List should not have element-type, as we can add any type to it.
    struct ListType final : Type
    {
        ~ListType() override = default;

        [[nodiscard]] bool matches(const Value& value) const override;

        [[nodiscard]] bool equals(const Type& otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;
    };

    struct FunctionType final : Type
    {
        explicit FunctionType(const std::vector<std::shared_ptr<Type>>& argumentTypes,
                              const std::shared_ptr<Type>& returnType, bool isVariadic = false);

        ~FunctionType() override = default;

        [[nodiscard]] const Type& getReturnType() const;

        [[nodiscard]] bool matches(const Value& value) const override;

        [[nodiscard]] bool matchesArgumentTypes(const std::vector<std::shared_ptr<Thunk>>& thunks) const;

        [[nodiscard]] bool matchesReturnType(const Value& value) const;

        [[nodiscard]] bool equals(const Type& otherType) const override;

        [[nodiscard]] std::string toString() const override;

        [[nodiscard]] bool isCallable() const override;

        [[nodiscard]] int arity() const;

    private:
        std::vector<std::shared_ptr<Type>> argumentTypes;
        std::shared_ptr<Type> returnType;
        bool isVariadic;
        std::shared_ptr<Type> variadicType;
    };

    [[nodiscard]] std::shared_ptr<Type> typeFromValue(const Value& value);
}
