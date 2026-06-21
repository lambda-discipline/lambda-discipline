#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>
#include <lbd/frontend/ast/statement/Statement.hpp>
#include <lbd/type/Type.hpp>

namespace lbd::frontend::ast::statement
{
  class SymbolDefinitionStatement final : public Statement
  {
  public:
    SymbolDefinitionStatement(const source::Range &range,
                              expression::IdentifierExpressionPtr symbolNameIdentifierExpressionPtr,
                              type::TypePtr symbolTypePtr, expression::ExpressionPtr expressionPtr) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] const expression::IdentifierExpression &getSymbolNameIdentifierExpression() const noexcept;

    [[nodiscard]] const expression::Expression &getExpression() const noexcept;

    [[nodiscard]] expression::ExpressionPtr releaseExpressionPtr() noexcept;

  private:
    expression::IdentifierExpressionPtr symbolNameIdentifierExpressionPtr;
    type::TypePtr symbolTypePtr;
    expression::ExpressionPtr expressionPtr;
  };
}
