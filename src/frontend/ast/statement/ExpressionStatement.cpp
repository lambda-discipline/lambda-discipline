#include <lbd/frontend/ast/statement/ExpressionStatement.hpp>

namespace lbd::frontend::ast::statement
{
  ExpressionStatement::ExpressionStatement(expression::ExpressionPtr expressionPtr) noexcept
    : Statement(AstNodeKind::EXPRESSION_STATEMENT, expressionPtr->getRange()),
      expressionPtr(std::move(expressionPtr)) {}

  void ExpressionStatement::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    expressionPtr->print(outputStream, indent);
  }

  const expression::Expression &ExpressionStatement::getExpression() const noexcept { return *expressionPtr; }
}