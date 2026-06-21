#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>
#include <lbd/frontend/ast/statement/Statement.hpp>

namespace lbd::frontend::ast::statement
{
  class ExpressionStatement final : public Statement
  {
  public:
    explicit ExpressionStatement(expression::ExpressionPtr expressionPtr) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] const expression::Expression &getExpression() const noexcept;

  private:
    expression::ExpressionPtr expressionPtr;
  };
}
