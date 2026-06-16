#include <filesystem>
#include <fmt/core.h>
#include <lbd/diagnostics/ContextGuard.hpp>
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
      fmt::format("While parsing `{}`.", lexer.getBuffer().name));

    while (lexer.hasNext())
    {
      switch (lexer.peek().kind)
      {
        case token::TokenKind::IDENTIFIER:
        {
          if (lexer.peek().lexeme == "use")
          {
            lexer.advance();
            assertToken(token::TokenKind::STRING);
            std::string path(lexer.next().lexeme);
            // ReSharper disable once CppTooWideScopeInitStatement
            auto includedAstNodes = parseFile(path, lexer.peek().range);
            for (auto &astNode: includedAstNodes) astNodes.emplace_back(std::move(astNode));
          } else
          {
            astNodes.push_back(std::make_unique<ast::AstNode>(parseDefinitionAstNode()));
          }
          break;
        }
        case token::TokenKind::STRING:
        case token::TokenKind::NUMBER:
        case token::TokenKind::BACKWARD_SLASH:
        case token::TokenKind::OPEN_PARENTHESIS:
          astNodes.push_back(std::make_unique<ast::AstNode>(ast::AstNode{parseExpression()}));
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

  void Parser::assertToken(const token::TokenKind expectedKind) const noexcept
  {
    if (lexer.peek().kind != expectedKind)
    {
      context.getDiagnosticEmitter().error(
        lexer.peek().range,
        diagnostics::DiagnosticId::PARSER_TOKEN_MISMATCH,
        token::tokenKindToString(expectedKind),
        token::tokenKindToString(lexer.peek().kind)
      );
    }
  }

  void Parser::assertAndAdvance(const token::TokenKind expectedKind) const noexcept
  {
    assertToken(expectedKind);
    lexer.advance();
  }

  std::vector<std::unique_ptr<ast::AstNode>> Parser::parseFile(const std::string &path,
                                                               const source::Range &range) const noexcept
  {
    // Circular Dependency or Duplicate Load.
    if (context.getBufferManager().isBufferLoaded(path)) { return {}; }

    lexer::Lexer innerLexer(context, context.getBufferManager().getBuffer(context.loadFile(path, range)));
    const Parser parser(context, innerLexer);
    return parser.parse();
  }

  ast::DefinitionAstNode Parser::parseDefinitionAstNode() const noexcept
  {
    // TODO: In future change this to range.
    const source::Location location = lexer.peek().range.begin;
    const ast::IdentifierAstNode definitionName = parseIdentifierAstNode();
    assertAndAdvance(token::TokenKind::COLON);
    const type::Type definitionType = parseType();
    assertAndAdvance(token::TokenKind::ASSIGNMENT);
    ast::Expression expression = parseExpression();
    return ast::DefinitionAstNode{definitionName, definitionType, std::move(expression), location};
  }

  ast::IdentifierAstNode Parser::parseIdentifierAstNode() const noexcept
  {
    assertToken(token::TokenKind::IDENTIFIER);
    const source::Location location = lexer.peek().range.begin;
    const std::string name = lexer.next().lexeme;
    return ast::IdentifierAstNode{std::string(name), location};
  }

  ast::StringAstNode Parser::parseStringAstNode() const noexcept
  {
    assertToken(token::TokenKind::STRING);
    const source::Location location = lexer.peek().range.begin;
    const std::string value = lexer.next().lexeme;
    // TODO: Relook whether we need to escape the string or not.
    return ast::StringAstNode{std::string(value), location};
  }

  ast::FloatAstNode Parser::parseNumberAstNode() const noexcept
  {
    assertToken(token::TokenKind::NUMBER);
    const source::Location location = lexer.peek().range.begin;
    const double value = std::stod(std::string(lexer.next().lexeme));
    return ast::FloatAstNode{value, location};
  }

  // TODO: Remove primitive and compound type altogether and create TypeAstNode.
  type::PrimitiveType Parser::parsePrimitiveTypeName() const noexcept
  {
    assertToken(token::TokenKind::IDENTIFIER);
    const std::string name = lexer.next().lexeme;
    // TODO: Perform this check through interpreter::type package.
    if (name == "Float")
    {
      return type::PrimitiveType{type::PrimitiveType::Type::Float};
    }
    if (name == "Str")
    {
      return type::PrimitiveType{type::PrimitiveType::Type::String};
    }
    if (name == "Any")
    {
      return type::PrimitiveType{type::PrimitiveType::Type::Any};
    }
    return type::PrimitiveType{type::PrimitiveType::Type::Custom, std::string(name)};
  }

  type::Type Parser::parseType() const noexcept
  {
    type::PrimitiveType typ = parsePrimitiveTypeName();
    std::vector types{typ};
    while (lexer.hasNext() && lexer.peek().kind == token::TokenKind::ARROW)
    {
      assertAndAdvance(token::TokenKind::ARROW);
      types.push_back(parsePrimitiveTypeName());
    }
    if (types.size() == 1) return typ;
    // Construct Right Associative AST
    type::Type currentType = types.back();
    for (int typeIndex = static_cast<int>(types.size()) - 2; typeIndex >= 0; --typeIndex)
    {
      auto nextCompoundType = std::make_shared<type::CompoundType>();
      nextCompoundType->leftType = types[typeIndex];
      nextCompoundType->rightType = currentType;
      currentType = nextCompoundType;
    }
    return currentType;
  }

  ast::Expression Parser::parseExpression() const noexcept
  {
    switch (lexer.peek().kind)
    {
      case token::TokenKind::IDENTIFIER: return ast::Expression(parseIdentifierAstNode());
      case token::TokenKind::STRING: return ast::Expression(parseStringAstNode());
      case token::TokenKind::NUMBER: return ast::Expression(parseNumberAstNode());
      case token::TokenKind::BACKWARD_SLASH: return ast::Expression(parseLambdaExpression());
      case token::TokenKind::OPEN_PARENTHESIS: return ast::Expression(parseFunctionApplication());
      default:
        context.getDiagnosticEmitter().error(
          lexer.peek().range,
          diagnostics::DiagnosticId::PARSER_UNEXPECTED_TOKEN, token::tokenKindToString(lexer.peek().kind)
        );
    }
  }

  ast::LambdaExpression Parser::parseLambdaExpression() const noexcept
  {
    const source::Location location = lexer.peek().range.begin;
    assertAndAdvance(token::TokenKind::BACKWARD_SLASH);
    const ast::IdentifierAstNode argument = parseIdentifierAstNode();
    assertAndAdvance(token::TokenKind::COLON);
    const type::Type argumentType = parseType();
    assertAndAdvance(token::TokenKind::DOT);
    ast::Expression expression = parseExpression();
    return ast::LambdaExpression{
      argument, argumentType, std::make_unique<ast::Expression>(std::move(expression)), location
    };
  }

  ast::FunctionApplication Parser::parseFunctionApplication() const noexcept
  {
    const source::Location location = lexer.peek().range.begin;
    assertAndAdvance(token::TokenKind::OPEN_PARENTHESIS);
    const ast::IdentifierAstNode functionName = parseIdentifierAstNode();
    std::vector<std::unique_ptr<ast::Expression>> arguments;
    while (lexer.peek().kind != token::TokenKind::CLOSE_PARENTHESIS)
    {
      arguments.push_back(std::make_unique<ast::Expression>(std::move(parseExpression())));
    }
    assertAndAdvance(token::TokenKind::CLOSE_PARENTHESIS);
    return ast::FunctionApplication{functionName, std::move(arguments), location};
  }
}