import numpy as np
np_array = np.arange(0, 10, dtype=float)
np.save("test_data.npy", np_array)
