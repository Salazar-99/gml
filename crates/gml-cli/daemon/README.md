# gmld

This is the daemon for gml. It is responsible for running `gml node|cluster delete` after
the timeout for that resource expires. It has a granularity of 1 minute.