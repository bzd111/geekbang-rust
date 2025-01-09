# from xunmia import *
from pprint import pprint

from xunmia import Indexer, InputConfig

indexer = Indexer("./fixtures/config.yml")
updater = indexer.get_updater()
f = open("./fixtures/wiki_00.xml")
data = f.read()
f.close()
input_config = InputConfig(
    "xml", [("$value", "content")], [("id", ("string", "number"))]
)
updater.update(data, input_config)
updater.commit()

result = indexer.search("数学", ["title", "content"], 5, 0)
pprint(result)
