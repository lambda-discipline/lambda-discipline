#include <lbd/frontend/ast/expression/LambdaExpression.hpp>

namespace lbd::frontend::ast::expression
{
  LambdaExpression::LambdaExpression(const source::Range &range,
                                     IdentifierExpressionPtr argumentIdentifierExpressionPtr,
                                     type::TypePtr argumentTypePtr, ExpressionPtr expressionPtr) noexcept
    : Expression(AstNodeKind::LAMBDA_EXPRESSION, range),
      argumentIdentifierExpressionPtr(std::move(argumentIdentifierExpressionPtr)),
      argumentTypePtr(std::move(argumentTypePtr)),
      expressionPtr(std::move(expressionPtr)) {}

  void LambdaExpression::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << '\\' << *argumentIdentifierExpressionPtr << ": " << *argumentTypePtr << '.' << std::endl;
    expressionPtr->print(outputStream, indent + 1);
  }

  const IdentifierExpression &LambdaExpression::getArgumentIdentifierExpression() const noexcept
  {
    return *argumentIdentifierExpressionPtr;
  }

  const Expression &LambdaExpression::getExpression() const noexcept { return *expressionPtr; }
}
