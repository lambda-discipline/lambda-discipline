#pragma once

#include <memory>
#include <vector>
#include <lbd/type/TypeKind.hpp>

namespace lbd::type
{
  class Type;
  using TypePtr = std::shared_ptr<Type>;

  struct TypePtrHash
  {
    size_t operator()(const TypePtr &t) const noexcept;
  };

  struct TypePtrEq
  {
    bool operator()(const TypePtr &a, const TypePtr &b) const noexcept;
  };

  // Usage: std::unordered_map<TypePtr, TypePtr, TypePtrHash, TypePtrEq> typeSubstitutionMap;

  struct Constraint
  {
    std::string className; // Eq, Ord, Show.
    TypePtr type; // Most likely will be a VARIABLE: a, b.

    [[nodiscard]] bool operator==(const Constraint &other) const noexcept;
  };

  class Type
  {
  public:
    // Factory functions.
    [[nodiscard]] static TypePtr named(const std::string &name);

    [[nodiscard]] static TypePtr variable(const std::string &name);

    [[nodiscard]] static TypePtr applied(const TypePtr &base, std::vector<TypePtr> arguments);

    [[nodiscard]] static TypePtr function(const TypePtr &from, const TypePtr &to);

    [[nodiscard]] static TypePtr qualified(std::vector<Constraint> constraints, const TypePtr &qualifiedType);

    [[nodiscard]] static TypePtr any();

    // Observers.
    [[nodiscard]] TypeKind getKind() const noexcept;

    [[nodiscard]] const std::string &getName() const noexcept;

    [[nodiscard]] const TypePtr &getBase() const noexcept;

    [[nodiscard]] const std::vector<TypePtr> &getArguments() const noexcept;

    [[nodiscard]] const TypePtr &getFrom() const noexcept;

    [[nodiscard]] const TypePtr &getTo() const noexcept;

    [[nodiscard]] const std::vector<Constraint> &getConstraints() const noexcept;

    [[nodiscard]] const TypePtr &getQualifiedType() const noexcept;

    // Comparison.
    [[nodiscard]] bool equals(const Type &other) const noexcept;

    [[nodiscard]] bool accepts(const Type &other) const noexcept;

    [[nodiscard]] bool operator==(const Type &other) const noexcept;

    [[nodiscard]] size_t hash() const noexcept;

    friend std::ostream &operator<<(std::ostream &, const Type &) noexcept;

  private:
    TypeKind kind;

    // NAMED, VARIABLE.
    std::string name;

    // APPLIED.
    TypePtr base;
    std::vector<TypePtr> arguments;

    // FUNCTION.
    TypePtr from;
    TypePtr to;

    // QUALIFIED.
    std::vector<Constraint> constraints;
    TypePtr qualifiedType;
  };
}