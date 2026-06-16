#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>

namespace lbd::frontend::ast::expression
{
  class IdentifierExpression final : Expression
  {
  public:
    IdentifierExpression(const source::Range &range, std::string name) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

  private:
    std::string name;
  };
}