#pragma once
#include <string>

inline std::string escape(const std::string& str) {
    std::string result;
    result.reserve(str.size());
    for (const char c : str) {
        switch (c) {
            case '\n': result += "\\n"; break;
            case '\t': result += "\\t"; break;
            case '\r': result += "\\r"; break;
            case '\\': result += "\\\\"; break;
            case '"':  result += "\\\""; break;
            case '\'': result += "\\\'"; break;
            default:
                // Printable ASCII characters
                if (c >= 32 && c <= 126) {
                    result.push_back(c);
                } else {
                    // Non-printable: use hex escape
                    char buf[5];
                    snprintf(buf, sizeof(buf), "\\x%02X", static_cast<unsigned char>(c));
                    result += buf;
                }
        }
    }
    return result;
}

inline std::string unescape_string(const std::string &str) {
    std::string result;
    result.reserve(str.size());
    for (size_t i = 0; i < str.size(); ++i) {
        if (str[i] == '\\' && i + 1 < str.size()) {
            switch (str[i + 1]) {
                case 'n': result.push_back('\n');
                    break;
                case 't': result.push_back('\t');
                    break;
                case 'r': result.push_back('\r');
                    break;
                case '\\': result.push_back('\\');
                    break;
                case '"': result.push_back('"');
                    break;
                default:
                    result.push_back(str[i + 1]); // Unknown escape: take character
            }
            ++i; // Skip the escaped char
        } else {
            result.push_back(str[i]);
        }
    }
    return result;
}