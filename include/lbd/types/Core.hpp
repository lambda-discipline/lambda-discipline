#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

// Forward declaration to avoid circular dependency
namespace lbd::runtime
{
  struct Value;
}

namespace lbd::types
{
  class Type;
  using TypePtr = std::shared_ptr<Type>;

  enum class TypeKind
  {
    // Basic types
    Named, // Number, String, Bool, etc.
    Variable, // Type variables: a, b, etc.
    Any, // Matches anything

    // Composite types
    Applied, // List<Number>, Map<String, Number>
    Function, // String -> Number

    // Functional programming features
    Qualified, // Eq a => a -> a (constraints)

    // Runtime-specific
    Closure, // Lambda function at runtime
    NativeFunction // Builtin function
  };

  enum class TypeTag
  {
    Float,
    String,
    Closure,
    NativeFunction,
    List,
    Any
  };

  [[nodiscard]] std::string typeTagToString(TypeTag tag) noexcept;

  struct Constraint
  {
    std::string className; // Eq, Ord, Show, etc.
    TypePtr type; // Usually a VARIABLE

    [[nodiscard]] bool operator==(const Constraint &other) const noexcept;
  };

  struct TypePtrHash
  {
    size_t operator()(const TypePtr &t) const noexcept;
  };

  struct TypePtrEq
  {
    bool operator()(const TypePtr &a, const TypePtr &b) const noexcept;
  };

  class Type
  {
  public:
    // Factory functions for all kinds
    [[nodiscard]] static TypePtr named(const std::string &name);

    [[nodiscard]] static TypePtr variable(const std::string &name);

    [[nodiscard]] static TypePtr applied(const TypePtr &base, std::vector<TypePtr> arguments);

    [[nodiscard]] static TypePtr function(const TypePtr &from, const TypePtr &to);

    [[nodiscard]] static TypePtr qualified(std::vector<Constraint> constraints, const TypePtr &qualifiedType);

    [[nodiscard]] static TypePtr any();

    [[nodiscard]] static TypePtr closure();

    [[nodiscard]] static TypePtr nativeFunction();

    // Type kind and components (compile-time focused)
    [[nodiscard]] TypeKind getKind() const noexcept;

    [[nodiscard]] const std::string &getName() const noexcept;

    [[nodiscard]] const TypePtr &getBase() const noexcept;

    [[nodiscard]] const std::vector<TypePtr> &getArguments() const noexcept;

    [[nodiscard]] const TypePtr &getFrom() const noexcept;

    [[nodiscard]] const TypePtr &getTo() const noexcept;

    [[nodiscard]] const std::vector<Constraint> &getConstraints() const noexcept;

    [[nodiscard]] const TypePtr &getQualifiedType() const noexcept;

    // Comparison
    [[nodiscard]] bool equals(const Type &other) const noexcept;

    [[nodiscard]] bool accepts(const Type &other) const noexcept;

    [[nodiscard]] bool operator==(const Type &other) const noexcept;

    [[nodiscard]] size_t hash() const noexcept;

    // Runtime type checking
    [[nodiscard]] bool matches(const runtime::Value &value) const noexcept;

    [[nodiscard]] TypeTag typeTag() const noexcept;

    /// Control whether type checking forces evaluation of lazy thunks
    [[nodiscard]] bool allowsHardCheck() const noexcept;

    void setAllowsHardCheck(bool check) noexcept;

    [[nodiscard]] bool getIsVariadic() const noexcept { return m_isVariadic; }

    void setIsVariadic(const bool variadic) noexcept { m_isVariadic = variadic; }

    // String representation
    friend std::ostream &operator<<(std::ostream &, const Type &) noexcept;

    Type() : m_kind(TypeKind::Any) {}

  private:
    TypeKind m_kind;

    // Named, Variable
    std::string m_name;

    // Applied
    TypePtr m_base;
    std::vector<TypePtr> m_arguments;

    // Function
    TypePtr m_from;
    TypePtr m_to;

    // Qualified
    std::vector<Constraint> m_constraints;
    TypePtr m_qualifiedType;

    /// Runtime control for whether to force evaluation of thunks for type checking.
    bool m_hardCheck = true;

    // Cached typeTag for runtime optimization.
    mutable TypeTag m_cachedTag = TypeTag::Any;
    mutable bool m_tagCached = false;

    // Runtime function information
    bool m_isVariadic = false;
  };
}
