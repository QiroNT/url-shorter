math.randomseed(os.time())
request = function() 
   url_path = "/" .. math.random(0, 9999)
   return wrk.format("GET", url_path)
end
