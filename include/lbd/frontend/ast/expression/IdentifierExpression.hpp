#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>

namespace lbd::frontend::ast::expression
{
  class IdentifierExpression;
  using IdentifierExpressionPtr = std::unique_ptr<IdentifierExpression>;

  class IdentifierExpression final : public Expression
  {
  public:
    IdentifierExpression(const source::Range &range, std::string name) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] const std::string &getName() const noexcept;

  private:
    std::string name;
  };
}
