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
    [[nodiscard]] explicit Parser(Context &context, lexer::Lexer &lexer);

    [[nodiscard]] std::vector<ast::AstNodePtr> parse() const;

  private:
    void expect(token::TokenKind kind) const;

    void consume(token::TokenKind kind) const;

    [[nodiscard]] std::vector<ast::AstNodePtr> parseFile(const std::string &path,
                                                         const source::Range &range) const;

    [[nodiscard]] ast::statement::StatementPtr parseSymbolDefinitionStatement() const;

    [[nodiscard]] ast::expression::ExpressionPtr parseExpression() const;

    [[nodiscard]] ast::expression::ExpressionPtr parseLambdaExpression() const;

    [[nodiscard]] ast::expression::ExpressionPtr parseFunctionApplicationExpression() const;

    [[nodiscard]] ast::expression::IdentifierExpressionPtr parseIdentifierExpression() const;

    [[nodiscard]] ast::expression::ExpressionPtr parseStringExpression() const;

    [[nodiscard]] ast::expression::ExpressionPtr parseNumberExpression() const;

    [[nodiscard]] type::TypePtr parseType() const;

    /// Tries to parse a qualified type, if failed fallbacks to parsing function type.
    [[nodiscard]] type::TypePtr parseQualifiedType() const;

    [[nodiscard]] type::TypePtr parseFunctionType() const;

    [[nodiscard]] type::TypePtr parseAppliedType() const;

    [[nodiscard]] type::TypePtr parsePrimaryType() const;

    Context &context;
    lexer::Lexer &lexer;
  };
}
