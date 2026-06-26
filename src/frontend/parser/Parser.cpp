#include <filesystem>
// ReSharper disable once CppUnusedIncludeDirective
#include <fmt/core.h>
#include <lbd/Error.hpp>
#include <lbd/diagnostics/ContextGuard.hpp>
#include <lbd/frontend/ast/expression/FunctionApplicationExpression.hpp>
#include <lbd/frontend/ast/expression/IdentifierExpression.hpp>
#include <lbd/frontend/ast/expression/LambdaExpression.hpp>
#include <lbd/frontend/ast/expression/NumberExpression.hpp>
#include <lbd/frontend/ast/expression/StringExpression.hpp>
#include <lbd/frontend/ast/statement/ExpressionStatement.hpp>
#include <lbd/frontend/ast/statement/SymbolDefinitionStatement.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/frontend/parser/Parser.hpp>

namespace lbd::frontend::parser
{
  Parser::Parser(Context &context, lexer::Lexer &lexer) noexcept : context(context), lexer(lexer) {}

  std::vector<std::unique_ptr<ast::AstNode>> Parser::parse() const noexcept
  {
    std::vector<std::unique_ptr<ast::AstNode>> astNodes;

    // Adding Context Guard for diagnostics.
    diagnostics::ContextGuard contextGuard(
      context.getDiagnosticEmitter(),
      lexer.peek().range,
      fmt::format("While parsing `{}`.", lexer.getBuffer().getName()));

    while (lexer.hasNext())
    {
      switch (lexer.peek().kind)
      {
        case token::TokenKind::IDENTIFIER:
        {
          if (lexer.peek().lexeme == "use")
          {
            lexer.advance();
            expect(token::TokenKind::STRING);
            std::string path(lexer.next().lexeme);
            // ReSharper disable once CppTooWideScopeInitStatement
            auto includedAstNodes = parseFile(path, lexer.peek().range);
            for (auto &astNode: includedAstNodes) astNodes.emplace_back(std::move(astNode));
          } else
          {
            astNodes.push_back(parseSymbolDefinitionStatement());
          }
          break;
        }
        case token::TokenKind::STRING:
        case token::TokenKind::NUMBER:
        case token::TokenKind::BACKWARD_SLASH:
        case token::TokenKind::OPEN_PARENTHESIS:
          astNodes.push_back(std::make_unique<ast::statement::ExpressionStatement>(parseExpression()));
          break;
        case token::TokenKind::END_OF_FILE: break;
        default:
          context.getDiagnosticEmitter().error(
            lexer.peek().range,
            diagnostics::DiagnosticId::PARSER_UNEXPECTED_TOKEN,
            token::tokenKindToString(lexer.peek().kind)
          );
      }
    }

    return astNodes;
  }

  void Parser::expect(const token::TokenKind kind) const noexcept
  {
    if (lexer.peek().kind != kind)
    {
      context.getDiagnosticEmitter().error(
        lexer.peek().range,
        diagnostics::DiagnosticId::PARSER_TOKEN_MISMATCH,
        token::tokenKindToString(kind),
        token::tokenKindToString(lexer.peek().kind)
      );
    }
  }

  void Parser::consume(const token::TokenKind kind) const noexcept
  {
    expect(kind);
    lexer.advance();
  }

  std::vector<ast::AstNodePtr> Parser::parseFile(const std::string &path, const source::Range &range) const noexcept
  {
    // Circular Dependency or Duplicate Load.
    try
    {
      if (context.getBufferManager().isBufferLoaded(path)) return {};
    } catch (const FileSystemError &e)
    {
      context.getDiagnosticEmitter().error(range, e.what());
    }

    lexer::Lexer innerLexer(context, context.getBufferManager().getBuffer(context.loadFile(path, range)));
    const Parser parser(context, innerLexer);
    return parser.parse();
  }

  ast::statement::StatementPtr Parser::parseSymbolDefinitionStatement() const noexcept
  {
    const source::Location begin = lexer.peek().range.getBegin();
    ast::expression::IdentifierExpressionPtr symbolNameIdentifierExpressionPtr = parseIdentifierExpression();
    consume(token::TokenKind::COLON);
    type::TypePtr symbolTypePtr = parseType();
    consume(token::TokenKind::ASSIGNMENT);
    ast::expression::ExpressionPtr expressionPtr = parseExpression();
    const auto range = source::Range(begin, expressionPtr->getRange().getEnd());
    return std::make_unique<ast::statement::SymbolDefinitionStatement>(
      range,
      std::move(symbolNameIdentifierExpressionPtr),
      std::move(symbolTypePtr),
      std::move(expressionPtr)
    );
  }

  ast::expression::ExpressionPtr Parser::parseExpression() const noexcept
  {
    switch (lexer.peek().kind)
    {
      case token::TokenKind::IDENTIFIER: return parseIdentifierExpression();
      case token::TokenKind::STRING: return parseStringExpression();
      case token::TokenKind::NUMBER: return parseNumberExpression();
      case token::TokenKind::BACKWARD_SLASH: return parseLambdaExpression();
      case token::TokenKind::OPEN_PARENTHESIS: return parseFunctionApplicationExpression();
      default:
        context.getDiagnosticEmitter().error(
          lexer.peek().range,
          diagnostics::DiagnosticId::PARSER_UNEXPECTED_TOKEN, token::tokenKindToString(lexer.peek().kind)
        );
    }
  }

  ast::expression::IdentifierExpressionPtr Parser::parseIdentifierExpression() const noexcept
  {
    expect(token::TokenKind::IDENTIFIER);
    const auto range = lexer.peek().range;
    const std::string name = lexer.next().lexeme;
    return std::make_unique<ast::expression::IdentifierExpression>(range, std::string(name));
  }

  ast::expression::ExpressionPtr Parser::parseStringExpression() const noexcept
  {
    expect(token::TokenKind::STRING);
    const auto range = lexer.peek().range;
    const std::string value = lexer.next().lexeme;
    // TODO: Relook whether we need to escape the string or not.
    return std::make_unique<ast::expression::StringExpression>(range, std::string(value));
  }

  ast::expression::ExpressionPtr Parser::parseNumberExpression() const noexcept
  {
    expect(token::TokenKind::NUMBER);
    const auto range = lexer.peek().range;
    const double value = std::stod(std::string(lexer.next().lexeme));
    return std::make_unique<ast::expression::NumberExpression>(range, value);
  }

  ast::expression::ExpressionPtr Parser::parseLambdaExpression() const noexcept
  {
    const source::Location begin = lexer.peek().range.getBegin();
    consume(token::TokenKind::BACKWARD_SLASH);
    ast::expression::IdentifierExpressionPtr argumentIdentifierExpressionPtr = parseIdentifierExpression();
    consume(token::TokenKind::COLON);
    type::TypePtr argumentTypePtr = parseType();
    consume(token::TokenKind::DOT);
    ast::expression::ExpressionPtr expressionPtr = parseExpression();
    const auto range = source::Range(begin, expressionPtr->getRange().getEnd());
    return std::make_unique<ast::expression::LambdaExpression>(
      range,
      std::move(argumentIdentifierExpressionPtr),
      std::move(argumentTypePtr),
      std::move(expressionPtr)
    );
  }

  ast::expression::ExpressionPtr Parser::parseFunctionApplicationExpression() const noexcept
  {
    const source::Location begin = lexer.peek().range.getBegin();
    consume(token::TokenKind::OPEN_PARENTHESIS);
    ast::expression::IdentifierExpressionPtr functionNameIdentifierExpressionPtr = parseIdentifierExpression();
    std::vector<ast::expression::ExpressionPtr> argumentPtrs;
    while (lexer.peek().kind != token::TokenKind::CLOSE_PARENTHESIS) argumentPtrs.push_back(parseExpression());
    const auto range = source::Range(begin, lexer.peek().range.getEnd());
    consume(token::TokenKind::CLOSE_PARENTHESIS);
    return std::make_unique<ast::expression::FunctionApplicationExpression>(
      range, std::move(functionNameIdentifierExpressionPtr),
      std::move(argumentPtrs)
    );
  }

  type::TypePtr Parser::parseType() const noexcept
  {
    return parseQualifiedType();
  }

  type::TypePtr Parser::parseQualifiedType() const noexcept
  {
    // TODO: Add support for parsing qualified types.
    return parseFunctionType();
  }

  type::TypePtr Parser::parseFunctionType() const noexcept
  {
    auto from = parseAppliedType();
    if (lexer.peek().kind != token::TokenKind::ARROW) return from;

    consume(token::TokenKind::ARROW);
    return type::Type::function(from, parseFunctionType());
  }

  type::TypePtr Parser::parseAppliedType() const noexcept
  {
    auto base = parsePrimaryType();
    if (lexer.peek().kind != token::TokenKind::LESS_THAN) return base;

    consume(token::TokenKind::LESS_THAN);
    std::vector<type::TypePtr> arguments;
    arguments.push_back(parseType());
    while (lexer.peek().kind == token::TokenKind::COMMA)
    {
      consume(token::TokenKind::COMMA);
      arguments.push_back(parseType());
    }
    consume(token::TokenKind::GREATER_THAN);

    return type::Type::applied(base, std::move(arguments));
  }

  type::TypePtr Parser::parsePrimaryType() const noexcept
  {
    if (lexer.peek().kind == token::TokenKind::OPEN_PARENTHESIS)
    {
      consume(token::TokenKind::OPEN_PARENTHESIS);
      auto type = parseType();
      consume(token::TokenKind::CLOSE_PARENTHESIS);
      return type;
    }

    const auto name = lexer.peek().lexeme;
    consume(token::TokenKind::IDENTIFIER);
    if (name == "Any") return type::Type::any();
    if (!name.empty() && std::islower(static_cast<unsigned char>(name[0]))) return type::Type::variable(name);
    return type::Type::named(name);
  }
}
