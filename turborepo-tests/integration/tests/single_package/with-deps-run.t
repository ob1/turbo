Setup
  $ . ${TESTDIR}/../../../helpers/setup.sh
  $ . ${TESTDIR}/../_helpers/setup_monorepo.sh $(pwd) single_package_deps

Check
  $ ${TURBO} run test
  \xe2\x80\xa2 Running test (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
  build: cache miss, executing cd720138f792fbca
  build: 
  build: > build
  build: > echo 'building' > foo
  build: 
  test: cache miss, executing 118e4fe7f2e841be
  test: 
  test: > test
  test: > [[ ( -f foo ) && $(cat foo) == 'building' ]]
  test: 
  
   Tasks:    2 successful, 2 total
  Cached:    0 cached, 2 total
    Time:\s*[\.0-9]+m?s  (re)
  
Run a second time, verify caching works because there is a config
  $ ${TURBO} run test
  \xe2\x80\xa2 Running test (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
  build: cache hit, replaying logs cd720138f792fbca
  build: 
  build: > build
  build: > echo 'building' > foo
  build: 
  test: cache hit, replaying logs 118e4fe7f2e841be
  test: 
  test: > test
  test: > [[ ( -f foo ) && $(cat foo) == 'building' ]]
  test: 
  
   Tasks:    2 successful, 2 total
  Cached:    2 cached, 2 total
    Time:\s*[\.0-9]+m?s >>> FULL TURBO (re)
  
Run with --output-logs=hash-only
  $ ${TURBO} run test --output-logs=hash-only
  \xe2\x80\xa2 Running test (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
  build: cache hit, suppressing logs cd720138f792fbca
  test: cache hit, suppressing logs 118e4fe7f2e841be
  
   Tasks:    2 successful, 2 total
  Cached:    2 cached, 2 total
    Time:\s*[\.0-9]+m?s >>> FULL TURBO (re)
  
Run with --output-logs=errors-only
  $ ${TURBO} run test --output-logs=errors-only
  \xe2\x80\xa2 Running test (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
  
   Tasks:    2 successful, 2 total
  Cached:    2 cached, 2 total
    Time:\s*[\.0-9]+m?s >>> FULL TURBO (re)
  
Run with --output-logs=none
  $ ${TURBO} run test --output-logs=none
  \xe2\x80\xa2 Running test (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
  
   Tasks:    2 successful, 2 total
  Cached:    2 cached, 2 total
    Time:\s*[\.0-9]+m?s >>> FULL TURBO (re)
  