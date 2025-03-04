package universe_test


import "testing"

option now = () => 2030-01-01T00:00:00Z

inData = "
#datatype,string,long,dateTime:RFC3339,double,string,string,string
#group,false,false,false,false,true,true,true
#default,_result,,,,,,
,result,table,_time,_value,_measurement,user,_field
,,0,2018-05-22T19:53:26Z,0,CPU,user1,a
,,0,2018-05-22T19:53:36Z,1,CPU,user1,a
,,1,2018-05-22T19:53:26Z,4,CPU,user2,a
,,1,2018-05-22T19:53:36Z,20,CPU,user2,a
,,1,2018-05-22T19:53:46Z,7,CPU,user2,a
,,1,2018-05-22T19:53:56Z,10,CPU,user2,a
,,2,2018-05-22T19:53:26Z,1,RAM,user1,b
,,2,2018-05-22T19:53:36Z,2,RAM,user1,b
,,2,2018-05-22T19:53:46Z,3,RAM,user1,b
,,2,2018-05-22T19:53:56Z,5,RAM,user1,b
,,3,2018-05-22T19:53:26Z,2,RAM,user2,b
,,3,2018-05-22T19:53:36Z,4,RAM,user2,b
,,3,2018-05-22T19:53:46Z,4,RAM,user2,b
,,3,2018-05-22T19:53:56Z,0,RAM,user2,b
,,3,2018-05-22T19:54:06Z,2,RAM,user2,b
,,3,2018-05-22T19:54:16Z,10,RAM,user2,b
"
outData = "
#datatype,string,long,string,dateTime:RFC3339,double,double,string,string,string
#group,false,false,true,false,false,false,true,false,true
#default,_result,,,,,,,,
,result,table,_measurement,_time,_value_left,_value_right,user_left,user_right,_field
,,0,CPU,2018-05-22T19:53:26Z,0,4,user1,user2,a
,,0,CPU,2018-05-22T19:53:36Z,1,20,user1,user2,b
,,1,RAM,2018-05-22T19:53:26Z,1,2,user1,user2,b
,,1,RAM,2018-05-22T19:53:36Z,2,4,user1,user2,b
,,1,RAM,2018-05-22T19:53:46Z,3,4,user1,user2,b
,,1,RAM,2018-05-22T19:53:56Z,5,0,user1,user2,b
"
t_join = (table=<-) => {
    left = table
        |> range(start: 2018-05-22T19:53:00Z, stop: 2018-05-22T19:55:00Z)
        |> drop(columns: ["_start", "_stop"])
        |> filter(fn: (r) => r.user == "user1")
        |> group(columns: ["user"])
    right = table
        |> range(start: 2018-05-22T19:53:00Z, stop: 2018-05-22T19:55:00Z)
        |> drop(columns: ["_start", "_stop"])
        |> filter(fn: (r) => r.user == "user2")
        |> group(columns: ["_measurement", "_field"])

    return join(tables: {left: left, right: right}, on: ["_time", "_measurement", "_field"])
}

test _join = () => ({input: testing.loadStorage(csv: inData), want: testing.loadMem(csv: outData), fn: t_join})
