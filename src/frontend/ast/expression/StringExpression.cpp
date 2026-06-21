#include <lbd/frontend/ast/expression/StringExpression.hpp>

namespace lbd::frontend::ast::expression
{
  StringExpression::StringExpression(const source::Range &range, std::string value) noexcept
    : Expression(AstNodeKind::STRING_EXPRESSION, range), value(std::move(value)) {}

  void StringExpression::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << '\"' << value << '\"';
  }


  const std::string &StringExpression::getValue() const noexcept { return value; }
}