math.randomseed(os.time())
request = function() 
   url_path = "/https://example.com/" .. math.random(0, 999999)
   return wrk.format("POST", url_path)
end
