#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>

namespace lbd::frontend::ast::expression
{
  IdentifierExpression::IdentifierExpression(const source::Range &range, std::string name) noexcept
    : Expression(AstNodeKind::IDENTIFIER_EXPRESSION, range), name(std::move(name)) {}

  void IdentifierExpression::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << name;
  }
}