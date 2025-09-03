#include <lbd/fe/lexer.h>
#include <lbd/logs.h>
#include <sstream>
#include <utility>
#include <fstream>

namespace fe::lexer {
    static options::Options options_v;

    char Lexer::peek() const {
        return pos < source.size() ? source[pos] : '\0';
    }

    char Lexer::get() {
        const char c = peek();
        if (c != '\0') {
            ++pos;
            if (c == '\n') {
                ++row;
                col = 1;
            } else {
                ++col;
            }
        }
        return c;
    }

    bool Lexer::is_eof() const {
        return pos >= source.size();
    }

    loc::Loc Lexer::get_cur_loc() const {
        return {row, col, filepath};
    }

    Lexer::Lexer(const std::string &filepath, FromFile, options::Options options_) : filepath(filepath) {
        std::ifstream ifs(filepath);
        options_v = options_;
        if (!ifs) {
            options_v.logger.error({}, "IO error: could not open file ", filepath);
        }
        std::ostringstream ss;
        ss << ifs.rdbuf();
        source = ss.str();
    }

    Lexer::Lexer(std::string str, FromRepl, const options::Options options_) : source(std::move(str)) {
        options_v = options_;
    }

    token::Token Lexer::next_token() {
        char c = peek();
        // Skip whitespace
        while (std::isspace(c)) {
            get(); // Consume ' '
            c = peek();
        }
        const loc::Loc cur_loc = get_cur_loc();
        if (is_eof()) {
            return {token::Eof(), cur_loc};
        }
        // Identifiers [a-zA-Z_][a-zA-Z0-9_]*
        if (std::isalpha(c) || c == '_') {
            const size_t start = pos;
            while (std::isalnum(c) || c == '_') {
                get();
                c = peek();
            }
            const std::string value = source.substr(start, pos - start);
            return {token::Iden{value}, cur_loc};
        }
        auto lex_float = [this, &c]() -> double {
            const size_t start = pos;
            while (std::isdigit(c)) {
                get();
                c = peek();
            }
            if (c == '.') {
                get(); // Consume '.'
                c = peek();
                while (std::isdigit(c)) {
                    get();
                    c = peek();
                }
            }
            const double value = std::stod(source.substr(start, pos - start));
            return value;
        };
        // Positive Float
        if (std::isdigit(c)) {
            return {token::Float(lex_float()), cur_loc};
        }
        // String Literal: " ... "
        if (c == '"') {
            get(); // Consume '"'
            const size_t start = pos;
            c = peek();
            while (c != '"' && !is_eof()) {
                get();
                c = peek();
            }
            const std::string value = source.substr(start, pos - start);
            if (c != '"') {
                options_v.logger.error(cur_loc, "syntax error: unbalanced quote");
            }
            get(); // Consume '"'
            return {token::String{value}, cur_loc};
        }
        // Symbols
        switch (c) {
            case ':':
                get();
                return {token::Colon{}, cur_loc};
            case '=':
                get();
                return {token::Equal{}, cur_loc};
            case '\\':
                get();
                return {token::BackwardSlash{}, cur_loc};
            case '.':
                get();
                return {token::Dot{}, cur_loc};
            case '(':
                get();
                return {token::OpenParen{}, cur_loc};
            case ')':
                get();
                return {token::CloseParen{}, cur_loc};
            case '-':
                get();
                c = peek();
                if (c == '>') {
                    get();
                    return {token::Arrow{}, cur_loc};
                }
                if (c == '-') {
                    // Comment
                    while (c != '\n' && !is_eof()) {
                        c = get();
                    }
                    // After skipping comment, return the next token
                    return next_token();
                }
                // Negative Float
                if (std::isdigit(c)) {
                    return {token::Float(-lex_float()), cur_loc};
                }
                break;
            default:
                break;
        }
        options_v.logger.error(cur_loc, "syntax error: unexpected character ", c);
    }

    std::vector<token::Token> Lexer::lex_all() {
        std::vector<token::Token> tokens;
        while (true) {
            token::Token tok = next_token();
            tokens.push_back(tok);
            if (std::holds_alternative<token::Eof>(tok.typ)) {
                break;
            }
        }
        return tokens;
    }
}
