#include <algorithm>
#include <lbd/frontend/ast/expression/FunctionApplicationExpression.hpp>

namespace lbd::frontend::ast::expression
{
  FunctionApplicationExpression::FunctionApplicationExpression(const source::Range &range,
                                                               IdentifierExpressionPtr
                                                               functionNameIdentifierExpressionPtr,
                                                               std::vector<ExpressionPtr> argumentPtrs)
    noexcept
    : Expression(AstNodeKind::FUNCTION_APPLICATION_EXPRESSION, range),
      functionNameIdentifierExpressionPtr(std::move(functionNameIdentifierExpressionPtr)),
      argumentPtrs(std::move(argumentPtrs)) {}

  void FunctionApplicationExpression::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << '(' << *functionNameIdentifierExpressionPtr;
    for (const auto &argument: argumentPtrs)
    {
      outputStream << ' ';
      argument->print(outputStream, 0);
    }
    outputStream << ')';
  }


  const IdentifierExpression &FunctionApplicationExpression::getFunctionNameIdentifierExpression() const noexcept
  {
    return *functionNameIdentifierExpressionPtr;
  }


  const std::vector<ExpressionPtr> &FunctionApplicationExpression::getArguments() const noexcept
  {
    return argumentPtrs;
  }
}
