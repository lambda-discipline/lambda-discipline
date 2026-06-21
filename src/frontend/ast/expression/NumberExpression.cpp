#include <lbd/frontend/ast/expression/NumberExpression.hpp>

namespace lbd::frontend::ast::expression
{
  NumberExpression::NumberExpression(const source::Range &range, const double value) noexcept
    : Expression(AstNodeKind::NUMBER_EXPRESSION, range), value(value) {}

  void NumberExpression::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << value;
  }

  double NumberExpression::getValue() const noexcept { return value; }
}