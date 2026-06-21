#pragma once

#include <memory>
#include <vector>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>

namespace lbd::frontend::ast::expression
{
  class FunctionApplicationExpression final : public Expression
  {
  public:
    // TODO: Update the design: instead of function-name callee should also be able to provide a lambda-expression.
    FunctionApplicationExpression(const source::Range &range,
                                  IdentifierExpressionPtr functionNameIdentifierExpressionPtr,
                                  std::vector<ExpressionPtr> argumentPtrs) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] const IdentifierExpression &getFunctionNameIdentifierExpression() const noexcept;

    [[nodiscard]] const std::vector<ExpressionPtr> &getArguments() const noexcept;

  private:
    IdentifierExpressionPtr functionNameIdentifierExpressionPtr;
    std::vector<ExpressionPtr> argumentPtrs;
  };
}
