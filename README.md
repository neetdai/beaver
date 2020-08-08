用rust重写nats的基本服务, 仅供学习rust.

感谢 [nkbai](https://github.com/nkbai)

参考文档: https://www.cnblogs.com/vimisky/p/4931015.html

### 性能测试

硬件配置:

    OS: Ubuntu 18.04 bionic
    Kernel: x86_64 Linux 4.15.0-112-generic
    CPU: Intel Core i5-6400 @ 4x 3.3GHz [27.8°C]
    RAM: 5058MiB / 7889MiB
 

测试命令:
    

单机性能测试结果:

百位订阅者

    ps auxw --sort=rss | grep beaver
    PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND

    25894  0.2  0.2 200024 21552 ?        S    14:28   0:00 /usr/lib/git-core/git-remote-https origin https://www.github.com/neetdai/beaver.git

    ./main -ns 100 -np 10 -ms 1000 foo

    Starting benchmark [msgs=100000, msgsize=1000, pubs=10, subs=100]
    NATS Pub/Sub stats: 988,911 msgs/sec ~ 943.10 MB/sec
    Pub stats: 9,832 msgs/sec ~ 9.38 MB/sec
    [1] 985 msgs/sec ~ 962.69 KB/sec (10000 msgs)
    [2] 986 msgs/sec ~ 963.68 KB/sec (10000 msgs)
    [3] 985 msgs/sec ~ 962.70 KB/sec (10000 msgs)
    [4] 986 msgs/sec ~ 963.00 KB/sec (10000 msgs)
    [5] 986 msgs/sec ~ 963.60 KB/sec (10000 msgs)
    [6] 987 msgs/sec ~ 963.89 KB/sec (10000 msgs)
    [7] 985 msgs/sec ~ 962.69 KB/sec (10000 msgs)
    [8] 987 msgs/sec ~ 964.38 KB/sec (10000 msgs)
    [9] 987 msgs/sec ~ 964.22 KB/sec (10000 msgs)
    [10] 988 msgs/sec ~ 965.36 KB/sec (10000 msgs)
    min 985 | avg 986 | max 988 | stddev 1 msgs
    Sub stats: 979,287 msgs/sec ~ 933.92 MB/sec
    [1] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [2] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [3] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [4] 9,835 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [5] 9,835 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [6] 9,851 msgs/sec ~ 9.39 MB/sec (100000 msgs)
    [7] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [8] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [9] 9,835 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [10] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [11] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [12] 9,835 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [13] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [14] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [15] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [16] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [17] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [18] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [19] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [20] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [21] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [22] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [23] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [24] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [25] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [26] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [27] 9,836 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [28] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [29] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [30] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [31] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [32] 9,835 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [33] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [34] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [35] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [36] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [37] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [38] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [39] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [40] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [41] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [42] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [43] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [44] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [45] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [46] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [47] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [48] 9,843 msgs/sec ~ 9.39 MB/sec (100000 msgs)
    [49] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [50] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [51] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [52] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [53] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [54] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [55] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [56] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [57] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [58] 9,850 msgs/sec ~ 9.39 MB/sec (100000 msgs)
    [59] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [60] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [61] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [62] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [63] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [64] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [65] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [66] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [67] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [68] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [69] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [70] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [71] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [72] 9,839 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [73] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [74] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [75] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [76] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [77] 9,838 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [78] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [79] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [80] 9,832 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [81] 9,831 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [82] 9,831 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [83] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [84] 9,834 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [85] 9,831 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [86] 9,833 msgs/sec ~ 9.38 MB/sec (100000 msgs)
    [87] 9,825 msgs/sec ~ 9.37 MB/sec (100000 msgs)
    [88] 9,825 msgs/sec ~ 9.37 MB/sec (100000 msgs)
    [89] 9,809 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    [90] 9,805 msgs/sec ~ 9.35 MB/sec (100000 msgs)
    [91] 9,806 msgs/sec ~ 9.35 MB/sec (100000 msgs)
    [92] 9,805 msgs/sec ~ 9.35 MB/sec (100000 msgs)
    [93] 9,812 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    [94] 9,809 msgs/sec ~ 9.35 MB/sec (100000 msgs)
    [95] 9,827 msgs/sec ~ 9.37 MB/sec (100000 msgs)
    [96] 9,822 msgs/sec ~ 9.37 MB/sec (100000 msgs)
    [97] 9,815 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    [98] 9,818 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    [99] 9,813 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    [100] 9,815 msgs/sec ~ 9.36 MB/sec (100000 msgs)
    min 9,805 | avg 9,831 | max 9,851 | stddev 7 msgs


千位订阅者(这里截取前10位订阅者, 不然文件太长)

    ps auxw --sort=rss | grep beaver
    PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
    1022  200  0.2 301092 17620 pts/4    Sl+  14:41   2:48 target/release/beaver

    ./main -ns 1000 -np 10 -ms 1000 foo

    Starting benchmark [msgs=100000, msgsize=1000, pubs=10, subs=1000]
    NATS Pub/Sub stats: 424,945 msgs/sec ~ 405.26 MB/sec
    Pub stats: 504 msgs/sec ~ 492.27 KB/sec
    [1] 50 msgs/sec ~ 49.70 KB/sec (10000 msgs)
    [2] 50 msgs/sec ~ 49.71 KB/sec (10000 msgs)
    [3] 50 msgs/sec ~ 49.71 KB/sec (10000 msgs)
    [4] 50 msgs/sec ~ 49.32 KB/sec (10000 msgs)
    [5] 50 msgs/sec ~ 49.28 KB/sec (10000 msgs)
    [6] 50 msgs/sec ~ 49.28 KB/sec (10000 msgs)
    [7] 50 msgs/sec ~ 49.27 KB/sec (10000 msgs)
    [8] 50 msgs/sec ~ 49.26 KB/sec (10000 msgs)
    [9] 50 msgs/sec ~ 49.26 KB/sec (10000 msgs)
    [10] 50 msgs/sec ~ 49.26 KB/sec (10000 msgs)
    min 50 | avg 50 | max 50 | stddev 0 msgs
    Sub stats: 424,523 msgs/sec ~ 404.86 MB/sec
    [1] 424 msgs/sec ~ 415.03 KB/sec (100000 msgs)
    [2] 424 msgs/sec ~ 415.00 KB/sec (100000 msgs)
    [3] 424 msgs/sec ~ 415.00 KB/sec (100000 msgs)
    [4] 424 msgs/sec ~ 415.00 KB/sec (100000 msgs)
    [5] 424 msgs/sec ~ 415.00 KB/sec (100000 msgs)
    [6] 424 msgs/sec ~ 415.01 KB/sec (100000 msgs)
    [7] 424 msgs/sec ~ 415.02 KB/sec (100000 msgs)
    [8] 424 msgs/sec ~ 415.03 KB/sec (100000 msgs)
    [9] 424 msgs/sec ~ 415.03 KB/sec (100000 msgs)
    [10] 424 msgs/sec ~ 415.00 KB/sec (100000 msgs)
    min 424 | avg 424 | max 425 | stddev 0 msgs
