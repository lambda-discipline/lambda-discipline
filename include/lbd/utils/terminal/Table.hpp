#pragma once

#include <string>
#include <vector>
#include <iomanip>
#include <iostream>
#include <lbd/utils/terminal/Colors.hpp>

namespace lbd::utils::terminal
{
    inline void printTableRow(const std::vector<std::string>& row, const std::vector<size_t>& widths,
                              const std::string& color = "") noexcept
    {
        std::cout << color;
        for (size_t i = 0; i < row.size(); ++i)
        {
            std::cout << std::setw(static_cast<int>(widths[i])) << std::left << row[i] << "  ";
        }
        std::cout << Colors().reset << std::endl;
    }

    inline void printTable(const std::vector<std::string>& headers, const std::vector<std::vector<std::string>>& data,
                           const std::string& color) noexcept
    {
        std::vector<size_t> widths(headers.size(), 0);
        for (size_t i = 0; i < headers.size(); ++i)
        {
            widths[i] = headers[i].size();
        }
        for (const auto& row : data)
        {
            for (size_t i = 0; i < row.size(); ++i)
            {
                if (row[i].size() > widths[i])
                {
                    widths[i] = row[i].size();
                }
            }
        }
        printTableRow(headers, widths, color);
        for (size_t i = 0; i < headers.size(); ++i)
        {
            std::cout << std::string(widths[i], '-') << "  ";
        }
        std::cout << std::endl;
        for (const auto& row : data)
        {
            printTableRow(row, widths);
        }
    }
}
