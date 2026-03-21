cmake_minimum_required(VERSION 3.20)
project(lambda-discipline)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

####### FRONTEND #######
file(GLOB_RECURSE FRONTEND_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/frontend/*.cpp
)
add_library(lbdFrontend STATIC)
target_sources(lbdFrontend PRIVATE ${FRONTEND_SOURCE_FILES})
target_include_directories(lbdFrontend PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

####### RUNTIME #######
file(GLOB_RECURSE RUNTIME_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/runtime/*.cpp
)
add_library(lbdRuntime STATIC)
target_sources(lbdRuntime PRIVATE ${RUNTIME_SOURCE_FILES})
target_include_directories(lbdRuntime PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

###### MAIN EXECUTABLE ######
file(GLOB LBD_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/*.cpp
)
add_executable(lbd)
target_sources(lbd PRIVATE ${LBD_SOURCE_FILES})
target_link_libraries(lbd PRIVATE
    lbdFrontend
    lbdRuntime
)

find_package(PkgConfig REQUIRED)
# Readline.
pkg_check_modules(READLINE REQUIRED readline)
target_include_directories(lbd PRIVATE ${READLINE_INCLUDE_DIRS})
target_link_libraries(lbd PRIVATE ${READLINE_LIBRARIES})

# Fully static build (platform dependent).
option(LBD_STATIC_BUILD "Build fully static binary" OFF)
if (LBD_STATIC_BUILD)
    target_link_options(lbd PRIVATE -static)
    # Readline depends upon ncurses.
    target_link_libraries(lbd PRIVATE ncurses tinfo)
endif ()

# Warnings.
target_compile_options(lbd PRIVATE -Wall -Wextra -Wpedantic)

# Tests.
enable_testing()

# Example test
# add_executable(lexer_test tests/lexer_test.cc)
# target_link_libraries(lexer_test PRIVATE lbdFrontend)
# add_test(NAME lexer_test COMMAND lexer_test)