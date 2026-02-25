MNIST_FILES := train-images-idx3-ubyte \
			train-labels-idx1-ubyte \
			t10k-images-idx3-ubyte \
			t10k-labels-idx1-ubyte
MNIST_PATHS := $(addsuffix .gz, $(MNIST_FILES))

mnist: $(addprefix mnist/, $(MNIST_FILES))

mnist/%: mnist/%.gz
	gzip -d mnist/$*.gz

mnist/%.gz:
	mkdir -p mnist
	curl -o $@ https://raw.githubusercontent.com/fgnt/mnist/master/$*.gz
