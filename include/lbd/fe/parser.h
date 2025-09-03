#pragma once

#include <lbd/options.h>
#include <lbd/fe/ast.h>
#include <lbd/fe/token.h>
#include <lbd/intp/types.h>
#include <vector>

namespace fe::parser {
    struct Parser {
        ast::Program program;

        explicit Parser(const std::vector<token::Token> &tokens, options::Options options_ = {});

    private:
        // TODO: Add checks for T to be a variant of fe::token::TokenType
        template<typename T>
        static void assert_token(const std::vector<token::Token> &tokens, size_t &i);

        template<typename T>
        static void assert_n_eat(const std::vector<token::Token> &tokens, size_t &i);

        static ast::IdenAstNode eat_iden(const std::vector<token::Token> &tokens, size_t &i);

        static intp::types::PrimitiveType eat_primitive_type_name(const std::vector<token::Token> &tokens, size_t &i);

        static intp::types::Type parse_type(const std::vector<token::Token> &tokens, size_t &i);

        static ast::Expression parse_expression(const std::vector<token::Token> &tokens, size_t &i);

        static ast::LambdaExpression parse_lambda_expression(const std::vector<token::Token> &tokens, size_t &i);

        static ast::FunctionApplication parse_function_application(const std::vector<token::Token> &tokens, size_t &i);

        static ast::DefAstNode parse_def_ast_node(const std::vector<token::Token> &tokens, size_t &i);

        static std::vector<ast::AstNode> build_ast(const std::vector<token::Token> &tokens, size_t &i);
    };
}
