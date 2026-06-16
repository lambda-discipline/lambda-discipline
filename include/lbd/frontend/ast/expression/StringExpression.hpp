#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>

namespace lbd::frontend::ast::expression
{
  class StringExpression final : Expression
  {
  public:
    StringExpression(const source::Range &range, std::string value) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

  private:
    std::string value;
  };
}