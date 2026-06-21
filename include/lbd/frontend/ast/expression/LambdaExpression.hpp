#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>
#include <lbd/type/Type.hpp>

namespace lbd::frontend::ast::expression
{
  class LambdaExpression final : public Expression
  {
  public:
    LambdaExpression(const source::Range &range, IdentifierExpressionPtr argumentIdentifierExpressionPtr,
                     type::TypePtr argumentTypePtr, ExpressionPtr expressionPtr) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] const IdentifierExpression &getArgumentIdentifierExpression() const noexcept;

    [[nodiscard]] const Expression &getExpression() const noexcept;

  private:
    IdentifierExpressionPtr argumentIdentifierExpressionPtr;
    type::TypePtr argumentTypePtr;
    ExpressionPtr expressionPtr;
  };
}
