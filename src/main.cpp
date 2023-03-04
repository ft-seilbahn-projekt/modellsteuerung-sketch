#include <Arduino.h>
#include <ftSwarm.h>
#include <FastLED.h>
#include <esp_task_wdt.h>

enum Ternary_t // 3-state logic
{
    True,
    False,
    Unknown
};

enum SwarmTypeElement_t
{
    Digital,
    Analog
};

class Node
{
public:
    char name[100];
    SwarmTypeElement_t type;

    FtSwarmSwitch *asSwitch;
    Ternary_t asSwitchState;

    FtSwarmAnalogInput *asAnalogInput;
    uint16_t asAnalogInputValue;
    uint16_t asAnalogThreshold;

    Node *next;
    bool has_next;
};

FtSwarmSerialNumber_t local;

Node *head;
Node *tail;
bool has_list_elems = false;

Ternary_t fromBool(bool in)
{
    if (in)
        return Ternary_t::True;
    else
        return Ternary_t::False;
}

int timedRead()
{
    int c;
    unsigned long _startMillis = millis();
    do
    {
        c = Serial.read();
        if (c >= 0)
        {
            return c;
        }
    } while (millis() - _startMillis < 3); // We require the sender to send a new char every 3ms
    return -1; // -1 indicates timeout
}

void appendNode(Node *node)
{
    if (!has_list_elems)
    {
        head = node;
        has_list_elems = true;
        tail = node;

        return;
    }

    tail->next = node;
    tail->has_next = true;
    tail = node;
}

void printNodes()
{
    if (!has_list_elems)
    {
        Serial.println("#debug nodes = []");
        return;
    }

    Serial.println("#debug nodes = [");

    Node *current = head;

    while (true)
    {
        Serial.printf("#debug '%s',\r\n", current->name);

        if (!current->has_next) break;

        current = current->next;
    }
    
    Serial.println("#debug ]");
}

void doDigital(Node *node) {
    Ternary_t actual = fromBool(node->asSwitch->getState());

    if (actual == node->asSwitchState) return;

    Serial.printf("!%s %d\r\n", node->name, node->asSwitch->getState());
    node->asSwitchState = actual;
}

void doAnalog(Node *node) {
    uint16_t actual = node->asAnalogInput->getValue();

    uint16_t diff = abs(actual - node->asAnalogInputValue);
    if (diff < node->asAnalogThreshold) return;

    node->asAnalogInputValue = actual;
    Serial.printf("!%s %d\r\n", node->name, actual);
}

void setup()
{
    Serial.begin(115200);

    Serial.print("Setup: Executing on core "); Serial.println(xPortGetCoreID());
    esp_task_wdt_init(30, false);

    ftSwarm.verbose(true);
    local = ftSwarm.begin();

    Serial.println(F(">>>"));
}

void loop()
{
    delay(25);

    // Work on Commands by the Controller
    if (Serial.available())
    {

        String command;
        int c = timedRead();
        while (c >= 0)
        {
            command += (char)c;
            c = timedRead();
        }

        command.replace("\r", "");
        command.replace("\n", "");

        String args = command.substring(command.indexOf(" "));


        if (command.startsWith("sub"))
        {
            command = command.substring(4);
            

            if (command.startsWith("digital"))
            {
                command = command.substring(8);
                Node *node = new Node();

                strcpy(node->name, command.c_str());
                node->type = SwarmTypeElement_t::Digital;
                node->asSwitch = new FtSwarmSwitch(command.c_str());
                node->asSwitchState = Unknown;
                node->has_next = false;

                appendNode(node);

                Serial.printf("#debug Subscribed to Button Press on %s\r\n", command.c_str());
                Serial.println("suc sub");
                return;
            } else if (command.startsWith("analog")) {
                command = command.substring(7);
                Node *node = new Node();

                auto toSplitAt = command.indexOf(" ");

                String threshold = command.substring(0, toSplitAt);
                command = command.substring(toSplitAt+1);

                strcpy(node->name, command.c_str());
                node->type = SwarmTypeElement_t::Analog;
                node->asAnalogInput = new FtSwarmAnalogInput(command.c_str());
                node->asAnalogInputValue = 0;
                node->asAnalogThreshold = threshold.toInt();
                node->has_next = false;

                appendNode(node);

                Serial.printf("#debug Subscribed to Analog Input on %s\r\n", command.c_str());
                Serial.println("suc sub");
                return;
            }

            Serial.println("err sub");
        }
        else if (command.startsWith("mot"))
        {
            
            command = command.substring(4);

            int index = command.indexOf(" ");
            String value = command.substring(index+1);
            command = command.substring(0, index);

            char cached_name[100];
            strcpy(cached_name, command.c_str());

            auto *mot = new FtSwarmMotor(cached_name);
            mot->setSpeed((int16_t) value.toInt());
            
            Serial.printf("suc mot %ld\r\n", value.toInt());
        }
        else if (command.startsWith("led")){

            
            if (command.startsWith("led on")) {
                for (int i = 2; i < 16; i++) {
                    auto *led = new FtSwarmLED(local, i);
                    led->setColor(CRGB::White);
                    led->setBrightness(50);
                }
            } else {
                for (int i = 2; i < 16; i++) {
                    auto *led = new FtSwarmLED(local, i);
                    led->setColor(CRGB::Black);
                    led->setBrightness(0);
                }
            }

            Serial.println("suc led");
        }
        else if (command.startsWith("otr"))
        {
            command = command.substring(4);

            int i = command.indexOf(" ");
            String nameIn = command.substring(0, i);
            command = command.substring(i);

            i = command.indexOf(" ");
            uint8_t actionIn = command.substring(0, i).toInt();
            command = command.substring(i);

            i = command.indexOf(" ");
            String nameOut = command.substring(0, i);
            command = command.substring(i);
            
            auto *in = new FtSwarmSwitch(nameIn.c_str());
            FtSwarmTrigger_t trigger = (actionIn == 0) ? FTSWARM_TRIGGERDOWN : FTSWARM_TRIGGERUP;
            auto *out = new FtSwarmMotor(nameOut.c_str());
            int valueOut = command.toInt();

            in->onTrigger(trigger, out, valueOut);
            Serial.println("suc otr");
        } 
        else if (command.startsWith("nod"))
        {
            printNodes();
            Serial.println("suc nod");
        }
        else if (command.startsWith("stp"))
        {
            ftSwarm.setup();
            Serial.println("suc stp");
        }else if (command.startsWith("res"))
        {
            ESP.restart();
        }
    }

    // Input Loop

    if (!has_list_elems) return;

    Node *current = head;

    while (true)
    {
        switch (current->type) {
            case Digital:
                doDigital(current);
                break;
            case Analog:
                doAnalog(current);
                break;
        }

        if (!current->has_next) break;

        current = current->next;
    }
}
