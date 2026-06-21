#pragma once

#include <lbd/frontend/ast/expression/Expression.hpp>

namespace lbd::frontend::ast::expression
{
  class NumberExpression final : public Expression
  {
  public:
    NumberExpression(const source::Range &range, double value) noexcept;

    void print(std::ostream &outputStream, size_t indent) const noexcept override;

    [[nodiscard]] double getValue() const noexcept;

  private:
    double value;
  };
}
