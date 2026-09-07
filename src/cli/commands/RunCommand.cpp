#include <lbd/Driver.hpp>
#include <lbd/cli/commands/RunCommand.hpp>
#include <lbd/source/BufferId.hpp>

namespace lbd::cli
{
  void RunCommand::configure(CLI::App &command)
  {
    command.name("run");
    command.description("Run a Lambda Discipline source file");
    command.add_option("file", filepath)->required();
  }

  int RunCommand::execute(Context &context)
  {
    const source::BufferId bufferId = context.getBufferManager().loadFile(filepath.string());
    auto result = Driver(context).run(bufferId);
    return EXIT_SUCCESS;
  }
}
