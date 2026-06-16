#pragma once

#include <CLI/App.hpp>
#include <lbd/Context.hpp>

namespace lbd::cli
{
  class ICommand
  {
  public:
    virtual ~ICommand() = default;

    virtual void configure(CLI::App &command) = 0;

    virtual int execute(Context &context) = 0;
  };
}