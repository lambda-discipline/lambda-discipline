cmake_minimum_required(VERSION 4.0)
project(lambda-discipline)

set(CMAKE_CXX_STANDARD 20)

include_directories(
    ${PROJECT_SOURCE_DIR}/include
)

file(GLOB_RECURSE FRONTEND_SOURCE_FILES CONFIGURE_DEPENDS ${PROJECT_SOURCE_DIR}/src/frontend/*.cpp)
add_library(frontend STATIC ${FRONTEND_SOURCE_FILES})

file(GLOB_RECURSE RUNTIME_SOURCE_FILES CONFIGURE_DEPENDS ${PROJECT_SOURCE_DIR}/src/runtime/*.cpp)
add_library(runtime STATIC ${RUNTIME_SOURCE_FILES})

file(GLOB LBD_SOURCE_FILES CONFIGURE_DEPENDS ${PROJECT_SOURCE_DIR}/src/*.cpp)
add_executable(lbd ${LBD_SOURCE_FILES})

target_link_libraries(lbd PRIVATE frontend runtime -static)

# TODO: Add build tests
# EXAMPLE: add_executable(lexer_test ../tests/lexer_test.cc)
#          target_link_libraries(lexer_test PRIVATE frontend)