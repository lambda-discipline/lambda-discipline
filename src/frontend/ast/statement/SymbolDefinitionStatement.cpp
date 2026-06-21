#include <lbd/frontend/ast/statement/SymbolDefinitionStatement.hpp>

namespace lbd::frontend::ast::statement
{
  SymbolDefinitionStatement::SymbolDefinitionStatement(const source::Range &range,
                                                       expression::IdentifierExpressionPtr
                                                       symbolNameIdentifierExpressionPtr,
                                                       type::TypePtr symbolTypePtr,
                                                       expression::ExpressionPtr expressionPtr) noexcept
    : Statement(AstNodeKind::SYMBOL_DEFINITION_STATEMENT, range),
      symbolNameIdentifierExpressionPtr(std::move(symbolNameIdentifierExpressionPtr)),
      symbolTypePtr(std::move(symbolTypePtr)), expressionPtr(std::move(expressionPtr)) {}

  void SymbolDefinitionStatement::print(std::ostream &outputStream, const size_t indent) const noexcept
  {
    printIndent(outputStream, indent);
    outputStream << *symbolNameIdentifierExpressionPtr << ": " << *symbolTypePtr << " = "
        << *expressionPtr << std::endl;
  }

  const expression::IdentifierExpression &SymbolDefinitionStatement::getSymbolNameIdentifierExpression() const noexcept
  {
    return *symbolNameIdentifierExpressionPtr;
  }

  const expression::Expression &SymbolDefinitionStatement::getExpression() const noexcept { return *expressionPtr; }

  expression::ExpressionPtr SymbolDefinitionStatement::releaseExpressionPtr() noexcept
  {
    return std::move(expressionPtr);
  }
}