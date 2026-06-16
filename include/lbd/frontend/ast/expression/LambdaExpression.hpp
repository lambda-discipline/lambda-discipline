#pragma once

#include <lbd/frontend/Type.hpp>
#include <lbd/frontend/ast/expression/Expression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>

namespace lbd::frontend::ast::expression
{
  class LambdaExpression final : Expression
  {
  public:
    LambdaExpression(const source::Range &range, std::string value) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

  private:
    IdentifierExpression argumentIdentifierExpression;
    type::Type argumentType;
    std::unique_ptr<Expression> expression;
    source::Location location;
  };
}