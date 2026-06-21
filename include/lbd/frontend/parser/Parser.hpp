#pragma once

#include <vector>
#include <lbd/Context.hpp>
#include <lbd/frontend/ast/AstNode.hpp>
#include <lbd/frontend/ast/expression/Expression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>
#include <lbd/frontend/ast/statement/Statement.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/type/Type.hpp>

namespace lbd::frontend::parser
{
  class Parser
  {
  public:
    [[nodiscard]] explicit Parser(Context &context, lexer::Lexer &lexer) noexcept;

    [[nodiscard]] std::vector<ast::AstNodePtr> parse() const noexcept;

  private:
    void expect(token::TokenKind kind) const noexcept;

    void consume(token::TokenKind kind) const noexcept;

    [[nodiscard]] std::vector<ast::AstNodePtr> parseFile(const std::string &path,
                                                         const source::Range &range) const noexcept;

    [[nodiscard]] ast::statement::StatementPtr parseSymbolDefinitionStatement() const noexcept;

    [[nodiscard]] ast::expression::ExpressionPtr parseExpression() const noexcept;

    [[nodiscard]] ast::expression::ExpressionPtr parseLambdaExpression() const noexcept;

    [[nodiscard]] ast::expression::ExpressionPtr parseFunctionApplicationExpression() const noexcept;

    [[nodiscard]] ast::expression::IdentifierExpressionPtr parseIdentifierExpression() const noexcept;

    [[nodiscard]] ast::expression::ExpressionPtr parseStringExpression() const noexcept;

    [[nodiscard]] ast::expression::ExpressionPtr parseNumberExpression() const noexcept;

    [[nodiscard]] type::TypePtr parseType() const noexcept;

    /// Tries to parse a qualified type, if failed fallbacks to parsing function type.
    [[nodiscard]] type::TypePtr parseQualifiedType() const noexcept;

    [[nodiscard]] type::TypePtr parseFunctionType() const noexcept;

    [[nodiscard]] type::TypePtr parseAppliedType() const noexcept;

    [[nodiscard]] type::TypePtr parsePrimaryType() const noexcept;

    Context &context;
    lexer::Lexer &lexer;
  };
}
